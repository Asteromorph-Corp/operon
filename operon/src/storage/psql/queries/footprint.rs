use std::str::FromStr;

use crate::schema::{RunFootprint, RunMetadata, RunState};
use crate::storage::StorageError;
use crate::storage::psql::client::StorageClient;
use crate::utils::GLOBAL;

impl StorageClient<'_> {
    /// Initializes the footprint table.
    pub async fn init_footprint(&self) -> Result<(), StorageError> {
        let schema_prefix = self.schema_prefix();

        let stmt = format!(
            "CREATE TABLE IF NOT EXISTS {schema_prefix}_footprint (
                key TEXT PRIMARY KEY DEFAULT '{GLOBAL}' CHECK (key = '{GLOBAL}'),
                run_id UUID NOT NULL UNIQUE,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                state TEXT NOT NULL DEFAULT 'running' CHECK (
                    state IN ('running', 'paused', 'completed', 'aborted')
                )
            )"
        );
        self.execute(&stmt, &[]).await?;
        Ok(())
    }

    /// Clears the footprint table.
    pub async fn clear_footprint(&self) -> Result<(), StorageError> {
        let schema_prefix = self.schema_prefix();
        let stmt = format!("TRUNCATE TABLE {schema_prefix}_footprint");
        self.execute(&stmt, &[]).await?;
        Ok(())
    }

    /// Gets a footprint value by key.
    pub async fn get_footprint(&self) -> Result<Option<RunFootprint>, StorageError> {
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
    pub async fn put_footprint(&self, footprint: &RunFootprint) -> Result<(), StorageError> {
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
