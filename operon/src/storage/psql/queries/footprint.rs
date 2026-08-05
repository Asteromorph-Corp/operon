use std::str::FromStr;

use indoc::formatdoc;

use crate::schema::{RunFootprint, RunMetadata, RunState};
use crate::storage::StorageError;
use crate::storage::psql::PsqlStorageResult;
use crate::storage::psql::client::StorageClient;
use crate::utils::{
    FOOTPRINT_VERSION, GLOBAL, SchemaPrefix, ShapeAction, ShapeRecord, build_tables,
    recorded_shape_query, sql_value_list,
};

/// The pointer to the footprint tables' shape ID.
const FOOTPRINT_RECORD: ShapeRecord<'static> = ShapeRecord {
    table: "_footprint_version",
    column: "version",
    id: FOOTPRINT_TABLE,
};
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
    /// If the backend carries a footprint table but without a version,
    /// we assume it predates footprint versioning and rebuild it.
    async fn footprint_action(&self, shape_id: &str) -> PsqlStorageResult<ShapeAction> {
        let schema_prefix = self.schema_prefix();
        let recorded_stmt = recorded_shape_query(FOOTPRINT_RECORD, schema_prefix);
        let recorded = self.query_opt(&recorded_stmt, &[]).await?;
        let action = ShapeAction::new(
            recorded
                .as_ref()
                .map(|row| row.get(FOOTPRINT_RECORD.column)),
            shape_id,
        );

        let present_stmt =
            format!("SELECT 1 WHERE to_regclass('{schema_prefix}{FOOTPRINT_TABLE}') IS NOT NULL;");
        let present = self.query_opt(&present_stmt, &[]).await?.is_some();

        Ok(match action {
            ShapeAction::Build if present => ShapeAction::Rebuild,
            action => action,
        })
    }

    /// Initializes the footprint table.
    ///
    /// Skips when the footprint table is already present and up to date.
    /// Destructively rebuilds when the footprint table is present but disagrees with the current
    /// version.
    pub async fn init_footprint(&self) -> PsqlStorageResult<()> {
        let schema_prefix = self.schema_prefix();
        let ShapeRecord { table, column, .. } = FOOTPRINT_RECORD;
        let shape_id = FOOTPRINT_VERSION.to_string();

        let init_record = formatdoc! {"
            CREATE TABLE IF NOT EXISTS {schema_prefix}{table} (
                id TEXT PRIMARY KEY,
                {column} TEXT NOT NULL
            );"
        };
        self.execute(&init_record, &[]).await?;

        let action = self.footprint_action(&shape_id).await?;
        if action == ShapeAction::Rebuild {
            tracing::warn!(
                "The last run was recorded under an incompatible version of Operon. \
                Progress from that run cannot be restored."
            );
        }

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
        Ok(())
    }

    /// Clears the footprint table.
    pub async fn clear_footprint(&self) -> PsqlStorageResult<()> {
        let schema_prefix = self.schema_prefix();
        let stmt = format!("TRUNCATE TABLE {schema_prefix}_footprint");
        self.execute(&stmt, &[]).await?;
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

        let stmt = format!(
            "INSERT INTO {schema_prefix}_footprint (key, run_id, updated_at, state)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (key) DO UPDATE SET run_id = EXCLUDED.run_id, updated_at = EXCLUDED.updated_at, state = EXCLUDED.state"
        );
        self.execute(&stmt, &[&GLOBAL, run_id, updated_at, &state])
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::utils::fixtures::{FootprintStore, footprint_shape, query_tokens};

    /// Asserts that the current footprint tables' DDL matches the fixture for the current
    /// [`FOOTPRINT_VERSION`].
    /// When this test fails due to an updated footprint shape, bump [`FOOTPRINT_VERSION`] and
    /// record the new shape in the corresponding version's fixture.
    #[test]
    fn test_init_footprint_query() {
        let stmt = init_footprint_query(SchemaPrefix(Some("test_data")));
        let recorded = footprint_shape(FootprintStore::Data);
        assert_eq!(query_tokens(&stmt), query_tokens(&recorded));
    }
}
