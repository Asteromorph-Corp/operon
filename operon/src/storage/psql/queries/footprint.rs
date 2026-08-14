use std::str::FromStr;

use chrono::{DateTime, Utc};
use indoc::formatdoc;
use uuid::Uuid;

use crate::schema::{RunFootprint, RunMetadata, RunState};
use crate::storage::StorageError;
use crate::storage::psql::PsqlStorageResult;
use crate::storage::psql::client::StorageClient;
use crate::utils::{
    FOOTPRINT_SHAPES, FOOTPRINT_VERSION, GLOBAL, SchemaPrefix, ShapeAction, ShapeRecord,
    build_tables, init_shape_table_query, shape_query, sql_value_list,
};

/// The pointer to the footprint table's shape ID.
const FOOTPRINT_RECORD: ShapeRecord<'static> = FOOTPRINT_SHAPES.record(FOOTPRINT_TABLE);
/// The table that records the footprint information.
const FOOTPRINT_TABLE: &str = "_footprint";

/// The statement building the footprint table.
fn init_footprint_query(schema_prefix: SchemaPrefix<'_>) -> String {
    let running = RunState::Running;
    let recorded = sql_value_list(RunState::RECORDED);

    formatdoc! {"
        CREATE TABLE IF NOT EXISTS {schema_prefix}_footprint (
            key TEXT PRIMARY KEY DEFAULT '{GLOBAL}' CHECK (key = '{GLOBAL}'),
            run_id UUID NOT NULL UNIQUE,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
            state TEXT NOT NULL DEFAULT '{running}' CHECK (
                state IN ({recorded})
            )
        );"
    }
}

impl StorageClient<'_> {
    /// The action to take when building the footprint table.
    ///
    /// A table with no recorded version predates versioning, so it is rebuilt.
    async fn footprint_action(&self, shape_id: &str) -> PsqlStorageResult<ShapeAction> {
        let stmt = shape_query(FOOTPRINT_RECORD, &[FOOTPRINT_TABLE], self.schema_prefix());
        let row = self.query_opt(&stmt, &[]).await?;
        Ok(ShapeAction::from_row(row.as_ref(), shape_id))
    }

    /// The recorded run's ID and timestamp.
    ///
    /// Skips the `state` column, which an older version may have written in a form this one
    /// cannot parse.
    async fn recorded_run(&self) -> PsqlStorageResult<Option<(Uuid, DateTime<Utc>)>> {
        let schema_prefix = self.schema_prefix();

        let stmt = format!(
            "SELECT run_id, updated_at FROM {schema_prefix}{FOOTPRINT_TABLE} WHERE key = $1"
        );
        let row = self.query_opt(&stmt, &[&GLOBAL]).await?;
        Ok(row.map(|row| (row.get(0), row.get(1))))
    }

    /// Initializes the footprint table.
    ///
    /// Skips when it is already up to date.
    /// Rebuilds it when it disagrees with the running footprint version, carrying the recorded run
    /// over as [`Aborted`](RunState::Aborted).
    pub async fn init_footprint(&self) -> PsqlStorageResult<()> {
        let schema_prefix = self.schema_prefix();
        let shape_id = FOOTPRINT_VERSION.to_string();

        let init_record = init_shape_table_query(FOOTPRINT_SHAPES, schema_prefix);
        self.execute(&init_record, &[]).await?;

        let action = self.footprint_action(&shape_id).await?;
        let carried = match action {
            ShapeAction::Rebuild => self.recorded_run().await?,
            _ => None,
        };
        if let Some(stmt) = build_tables(
            FOOTPRINT_RECORD,
            &[FOOTPRINT_TABLE],
            &shape_id,
            schema_prefix,
            action,
            init_footprint_query(schema_prefix),
        ) {
            self.batch_execute(&stmt).await?;
        }

        if let Some((run_id, at)) = carried {
            let footprint = RunFootprint::at(run_id, RunState::Aborted, at);
            self.put_footprint(&footprint).await?;
        }
        Ok(())
    }

    /// Gets a footprint value by key.
    pub async fn get_footprint(&self) -> PsqlStorageResult<Option<RunFootprint>> {
        let schema_prefix = self.schema_prefix();

        let stmt = format!(
            "SELECT run_id, updated_at, state FROM {schema_prefix}_footprint WHERE key = $1"
        );
        let Some(row) = self.query_opt(&stmt, &[&GLOBAL]).await? else {
            return Ok(None);
        };

        let run_id = row.get(0);
        let updated_at = row.get(1);
        let state = RunState::from_str(row.get(2)).map_err(StorageError::InvalidRunState)?;

        let footprint = RunFootprint {
            metadata: RunMetadata { run_id, state },
            at: updated_at,
        };
        Ok(Some(footprint))
    }

    /// Sets a footprint key-value pair.
    pub async fn put_footprint(&self, footprint: &RunFootprint) -> PsqlStorageResult<()> {
        let schema_prefix = self.schema_prefix();
        let run_id = &footprint.metadata.run_id;
        let updated_at = &footprint.at;
        let state = footprint.metadata.state.to_string();

        let stmt = formatdoc! {"
            INSERT INTO {schema_prefix}_footprint (key, run_id, updated_at, state)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (key) DO UPDATE SET
                run_id = EXCLUDED.run_id,
                updated_at = EXCLUDED.updated_at,
                state = EXCLUDED.state"
        };
        self.execute(&stmt, &[&GLOBAL, run_id, updated_at, &state])
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
        let stmt = init_footprint_query(SchemaPrefix(Some("test_data")));
        assert_footprint_shape(FootprintStore::Data, &stmt);
    }
}
