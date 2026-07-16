use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use uuid::Uuid;

use crate::meta_storage::mem::error::{MemResult, poisoned};
use crate::meta_storage::mem::resolution::ResolutionTable;
use crate::meta_storage::mem::ticket::TicketTable;
use crate::schema::RunFootprint;

/// The in-memory metadata store.
///
/// Ticket and resolution tables are keyed by job/dimension id. Their coordinates carry their arity
/// at runtime, so one store holds tables spanning jobs of differing arity without a type parameter.
///
/// # Concurrency
///
/// A table takes one lock over all of its rows, so writes to the same job serialize; only writes
/// to different jobs proceed in parallel. Sharding a table is deferred until a profile of the
/// rebuild in #64 shows the contention is worth it — that path drives every write from a single
/// task, so it takes the lock uncontended.
#[derive(Default)]
pub(super) struct MemStore {
    tickets: RwLock<HashMap<&'static str, Arc<TicketTable>>>,
    resolutions: RwLock<HashMap<&'static str, Arc<ResolutionTable>>>,
    footprint: RwLock<Option<RunFootprint>>,
    executions: RwLock<HashMap<Uuid, Execution>>,
}

/// One recorded execution of a run.
struct Execution {
    run_id: Uuid,
    ended_at: Option<chrono::DateTime<chrono::Utc>>,
    end_reason: Option<String>,
}

impl MemStore {
    /// Registers a job's ticket table, keeping an existing one.
    pub(super) fn init_ticket_table(&self, job_id: &'static str) -> MemResult<()> {
        self.tickets
            .write()
            .map_err(poisoned)?
            .entry(job_id)
            .or_insert_with(|| Arc::new(TicketTable::default()));
        Ok(())
    }

    /// Returns a job's ticket table, if it has been registered.
    pub(super) fn ticket_table(&self, job_id: &'static str) -> MemResult<Option<Arc<TicketTable>>> {
        Ok(self.tickets.read().map_err(poisoned)?.get(job_id).cloned())
    }

    /// Registers a dimension's resolution table, keeping an existing one.
    pub(super) fn init_resolution_table(&self, dim_id: &'static str) -> MemResult<()> {
        self.resolutions
            .write()
            .map_err(poisoned)?
            .entry(dim_id)
            .or_insert_with(|| Arc::new(ResolutionTable::default()));
        Ok(())
    }

    /// Returns a dimension's resolution table, if it has been registered.
    pub(super) fn resolution_table(
        &self,
        dim_id: &'static str,
    ) -> MemResult<Option<Arc<ResolutionTable>>> {
        Ok(self
            .resolutions
            .read()
            .map_err(poisoned)?
            .get(dim_id)
            .cloned())
    }

    pub(super) fn clear_footprint(&self) -> MemResult<()> {
        *self.footprint.write().map_err(poisoned)? = None;
        self.executions.write().map_err(poisoned)?.clear();
        Ok(())
    }

    pub(super) fn get_footprint(&self) -> MemResult<Option<RunFootprint>> {
        Ok(self.footprint.read().map_err(poisoned)?.clone())
    }

    pub(super) fn upsert_run(&self, footprint: &RunFootprint) -> MemResult<()> {
        *self.footprint.write().map_err(poisoned)? = Some(footprint.clone());
        Ok(())
    }

    pub(super) fn put_execution(&self, run_id: Uuid, execution_id: Uuid) -> MemResult<()> {
        self.executions.write().map_err(poisoned)?.insert(
            execution_id,
            Execution {
                run_id,
                ended_at: None,
                end_reason: None,
            },
        );
        Ok(())
    }

    pub(super) fn update_execution_on_finish(
        &self,
        footprint: &RunFootprint,
        execution_id: Uuid,
    ) -> MemResult<()> {
        let mut executions = self.executions.write().map_err(poisoned)?;
        let Some(execution) = executions.get_mut(&execution_id) else {
            return Ok(());
        };
        if execution.run_id != footprint.metadata.run_id {
            return Ok(());
        }
        execution.ended_at = Some(footprint.at);
        execution.end_reason = Some(footprint.metadata.state.to_string());
        Ok(())
    }
}
