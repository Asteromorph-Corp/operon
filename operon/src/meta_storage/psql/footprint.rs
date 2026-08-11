//! Operations for footprinting the PSQL metadata storage.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use indoc::formatdoc;
use uuid::Uuid;

use crate::meta_storage::MetaStorageError;
use crate::meta_storage::psql::PsqlClient;
use crate::meta_storage::psql::error::PsqlResult;
use crate::schema::{RunFootprint, RunState, TableShape};
use crate::utils::{
    FOOTPRINT_VERSION, GLOBAL, SchemaPrefix, ShapeAction, ShapeRecord, build_tables,
    footprint_record, init_shape_record_query, shape_query, sql_value_list,
};

/// The pointer to the footprint tables' shape ID.
const FOOTPRINT_RECORD: ShapeRecord<'static> = footprint_record(RUNS_TABLE);
/// The table that records the footprint information.
const RUNS_TABLE: &str = "runs";
/// All tables to be initialized for footprinting.
const FOOTPRINT_TABLES: [&str; 2] = ["run_executions", RUNS_TABLE];

/// The footprint tables' DDL.
fn init_footprint_query(schema_prefix: SchemaPrefix<'_>) -> String {
    let running = RunState::Running;
    let recorded = sql_value_list(RunState::RECORDED);
    let ended = sql_value_list(RunState::ENDED);

    formatdoc! {"
        CREATE TABLE IF NOT EXISTS {schema_prefix}runs (
            key TEXT PRIMARY KEY CHECK (key = '{GLOBAL}'),
            run_id UUID NOT NULL UNIQUE,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
            finished_at TIMESTAMP WITH TIME ZONE DEFAULT NULL,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
            state TEXT NOT NULL DEFAULT '{running}' CHECK (
                state IN ({recorded})
            )
        );

        CREATE TABLE IF NOT EXISTS {schema_prefix}run_executions (
            execution_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            run_id UUID REFERENCES {schema_prefix}runs(run_id) ON DELETE CASCADE,
            started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
            ended_at TIMESTAMP WITH TIME ZONE,
            end_reason TEXT CHECK (
                end_reason IN ({ended})
            )
        );"
    }
}

impl PsqlClient<'_> {
    /// The action to take when building the footprint tables.
    ///
    /// Tables with no recorded version predate versioning, so they are rebuilt.
    async fn footprint_action(&self, shape_id: &str) -> PsqlResult<ShapeAction> {
        let stmt = shape_query(FOOTPRINT_RECORD, &FOOTPRINT_TABLES, self.schema_prefix());
        let row = self.query_opt(&stmt, &[]).await?;
        Ok(ShapeAction::from_row(row.as_ref(), shape_id))
    }

    /// The recorded run's ID and timestamp.
    ///
    /// Skips the `state` column, which an older version may have written in a form this one
    /// cannot parse.
    async fn recorded_run(&self) -> PsqlResult<Option<(Uuid, DateTime<Utc>)>> {
        let schema_prefix = self.schema_prefix();

        let stmt =
            format!("SELECT run_id, updated_at FROM {schema_prefix}{RUNS_TABLE} WHERE key = $1");
        let row = self.query_opt(&stmt, &[&GLOBAL]).await?;
        Ok(row.map(|row| (row.get(0), row.get(1))))
    }

    /// Initializes the footprint tables.
    ///
    /// Skips when they are already up to date.
    /// Rebuilds them when they disagree with [`FOOTPRINT_VERSION`], carrying the recorded run over
    /// as [`Aborted`](RunState::Aborted) and dropping its execution history.
    pub async fn init_footprint(&self) -> PsqlResult<TableShape> {
        let schema_prefix = self.schema_prefix();
        let shape_id = FOOTPRINT_VERSION.to_string();

        let init_record = init_shape_record_query(FOOTPRINT_RECORD, schema_prefix);
        self.execute(&init_record, &[]).await?;

        let action = self.footprint_action(&shape_id).await?;
        let carried = match action {
            ShapeAction::Rebuild => self.recorded_run().await?,
            _ => None,
        };
        if action == ShapeAction::Rebuild {
            tracing::warn!(
                "The last run was recorded under an incompatible version of Operon. \
                Progress from that run cannot be gracefully resumed."
            );
        }

        if let Some(stmt) = build_tables(
            FOOTPRINT_RECORD,
            &FOOTPRINT_TABLES,
            &shape_id,
            schema_prefix,
            action,
            init_footprint_query(schema_prefix),
        ) {
            self.batch_execute(&stmt).await?;
        }

        if let Some((run_id, at)) = carried {
            let footprint = RunFootprint::at(run_id, RunState::Aborted, at);
            self.upsert_run(&footprint).await?;
        }
        Ok(action.into())
    }

    /// Clears the footprint table.
    pub async fn clear_footprint(&self) -> PsqlResult<()> {
        let schema_prefix = self.schema_prefix();

        let stmt = format!("TRUNCATE TABLE {schema_prefix}runs CASCADE");
        self.execute(&stmt, &[]).await?;
        Ok(())
    }

    /// Gets the run metadata.
    pub async fn get_footprint(&self) -> PsqlResult<Option<RunFootprint>> {
        let schema_prefix = self.schema_prefix();

        let stmt =
            format!("SELECT run_id, updated_at, state FROM {schema_prefix}runs WHERE key = $1");
        let Some(row) = self.query_opt(&stmt, &[&GLOBAL]).await? else {
            return Ok(None);
        };

        let run_id = row.get(0);
        let updated_at = row.get(1);
        let state = RunState::from_str(row.get(2)).map_err(MetaStorageError::InvalidRunState)?;

        Ok(Some(RunFootprint::at(run_id, state, updated_at)))
    }

    /// Updates the run table on run state change.
    pub async fn upsert_run(&self, footprint: &RunFootprint) -> PsqlResult<()> {
        let schema_prefix = self.schema_prefix();
        let run_id = footprint.metadata.run_id;
        let run_state = footprint.metadata.state.to_string();
        let updated_at = &footprint.at;
        let completed = RunState::Completed;

        let stmt = formatdoc! {"
            INSERT INTO {schema_prefix}runs (key, run_id, state, updated_at, finished_at)
            VALUES ($1, $2, $3, $4, CASE WHEN $3 = '{completed}' THEN $4 ELSE NULL::timestamptz END)
            ON CONFLICT (key) DO UPDATE SET
                run_id = EXCLUDED.run_id,
                state = EXCLUDED.state,
                updated_at = EXCLUDED.updated_at,
                finished_at = EXCLUDED.finished_at"
        };

        self.execute(&stmt, &[&GLOBAL, &run_id, &run_state, updated_at])
            .await?;

        Ok(())
    }

    /// Inserts a new execution row on execution start.
    pub async fn put_execution(&self, run_id: Uuid, execution_id: Uuid) -> PsqlResult<()> {
        let schema_prefix = self.schema_prefix();

        let stmt = formatdoc! {"
            INSERT INTO {schema_prefix}run_executions (run_id, execution_id)
            VALUES ($1, $2)"
        };
        self.execute(&stmt, &[&run_id, &execution_id]).await?;
        Ok(())
    }

    /// Updates the execution row on execution end.
    pub async fn update_execution_on_finish(
        &self,
        footprint: &RunFootprint,
        execution_id: Uuid,
    ) -> PsqlResult<()> {
        let schema_prefix = self.schema_prefix();
        let run_id = &footprint.metadata.run_id;
        let at = &footprint.at;
        let end_reason = footprint.metadata.state.to_string();

        let stmt = formatdoc! {"
            UPDATE {schema_prefix}run_executions
            SET ended_at = $3, end_reason = $4
            WHERE run_id = $1 AND execution_id = $2"
        };
        self.execute(&stmt, &[run_id, &execution_id, at, &end_reason])
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::fixtures::{FootprintStore, assert_footprint_shape};

    #[test]
    fn test_init_footprint_query() {
        let stmt = init_footprint_query(SchemaPrefix(Some("test_meta")));
        assert_footprint_shape(FootprintStore::Meta, &stmt);
    }
}
