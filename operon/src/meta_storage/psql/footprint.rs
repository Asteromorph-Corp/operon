//! Operations for footprinting the PSQL metadata storage.

use std::str::FromStr;

use uuid::Uuid;

use crate::meta_storage::MetaStorageError;
use crate::meta_storage::psql::PsqlClient;
use crate::meta_storage::psql::error::PsqlResult;
use crate::schema::{RunFootprint, RunState};
use crate::utils::GLOBAL;

impl PsqlClient<'_> {
    /// Initializes the footprint table.
    pub async fn init_footprint(&self) -> PsqlResult<()> {
        let schema_prefix = self.schema_prefix();

        let stmt = format!(
            "CREATE TABLE IF NOT EXISTS {schema_prefix}runs (
                key TEXT PRIMARY KEY CHECK (key = '{GLOBAL}'),
                run_id UUID NOT NULL UNIQUE,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                finished_at TIMESTAMP WITH TIME ZONE DEFAULT NULL,
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                state TEXT NOT NULL DEFAULT 'running' CHECK (
                    state IN ('running', 'paused', 'completed', 'aborted')
                )
            );

            CREATE TABLE IF NOT EXISTS {schema_prefix}run_executions (
                execution_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                run_id UUID REFERENCES {schema_prefix}runs(run_id) ON DELETE CASCADE,
                started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                ended_at TIMESTAMP WITH TIME ZONE,
                end_reason TEXT CHECK (
                    end_reason IN (NULL, 'paused', 'completed', 'aborted')
                )
            );"
        );

        self.batch_execute(&stmt).await?;
        Ok(())
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

        let stmt = format!(
            "INSERT INTO {schema_prefix}runs (key, run_id, state, updated_at, finished_at)
            VALUES ($1, $2, $3, $4, CASE WHEN $3 IN ('completed') THEN $4 ELSE NULL::timestamptz END)
            ON CONFLICT (key) DO UPDATE SET
                run_id = EXCLUDED.run_id,
                state = EXCLUDED.state,
                updated_at = EXCLUDED.updated_at,
                finished_at = EXCLUDED.finished_at"
        );

        self.execute(&stmt, &[&GLOBAL, &run_id, &run_state, updated_at])
            .await?;

        Ok(())
    }

    /// Inserts a new execution row on execution start.
    pub async fn put_execution(&self, run_id: Uuid, execution_id: Uuid) -> PsqlResult<()> {
        let schema_prefix = self.schema_prefix();

        let stmt = format!(
            "INSERT INTO {schema_prefix}run_executions (run_id, execution_id)
            VALUES ($1, $2)"
        );
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

        let stmt = format!(
            "UPDATE {schema_prefix}run_executions
            SET ended_at = $3, end_reason = $4
            WHERE run_id = $1 AND execution_id = $2"
        );
        self.execute(&stmt, &[run_id, &execution_id, at, &end_reason])
            .await?;
        Ok(())
    }
}
