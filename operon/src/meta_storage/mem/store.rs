use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use uuid::Uuid;

use crate::meta_storage::MetaStorageError;
use crate::meta_storage::mem::error::{MemResult, POISONED};
use crate::meta_storage::mem::resolution::ResolutionTable;
use crate::meta_storage::mem::ticket::TicketTable;
use crate::schema::RunFootprint;

/// The in-memory metadata store.
///
/// Ticket and resolution tables are keyed by job/dimension id in a type map: their arity `N` is a
/// const parameter, so a store holding tables of differing arity erases each to `dyn Any` on the
/// way in and downcasts it back at the [`MemClient`](super::MemClient) boundary, where `N` is
/// known again.
///
/// # Concurrency
///
/// A table takes one lock over all of its rows, so writes to the same job serialize; only writes
/// to different jobs proceed in parallel. Sharding a table is deferred until a profile of the
/// rebuild in #64 shows the contention is worth it — that path drives every write from a single
/// task, so it takes the lock uncontended.
#[derive(Default)]
pub(super) struct MemStore {
    tickets: RwLock<HashMap<&'static str, Arc<dyn Any + Send + Sync>>>,
    resolutions: RwLock<HashMap<&'static str, Arc<dyn Any + Send + Sync>>>,
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
    pub(super) fn init_ticket_table<const N: usize>(&self, job_id: &'static str) {
        self.tickets
            .write()
            .expect(POISONED)
            .entry(job_id)
            .or_insert_with(|| Arc::new(TicketTable::<N>::default()));
    }

    /// Returns a job's ticket table, if it has been registered.
    pub(super) fn ticket_table<const N: usize>(
        &self,
        job_id: &'static str,
    ) -> MemResult<Option<Arc<TicketTable<N>>>> {
        let tables = self.tickets.read().expect(POISONED);
        let Some(table) = tables.get(job_id) else {
            return Ok(None);
        };
        table
            .clone()
            .downcast::<TicketTable<N>>()
            .map(Some)
            .map_err(|_| MetaStorageError::Internal("ticket table registered with another arity"))
    }

    /// Registers a dimension's resolution table, keeping an existing one.
    pub(super) fn init_resolution_table<const N: usize>(&self, dim_id: &'static str) {
        self.resolutions
            .write()
            .expect(POISONED)
            .entry(dim_id)
            .or_insert_with(|| Arc::new(ResolutionTable::<N>::default()));
    }

    /// Returns a dimension's resolution table, if it has been registered.
    pub(super) fn resolution_table<const N: usize>(
        &self,
        dim_id: &'static str,
    ) -> MemResult<Option<Arc<ResolutionTable<N>>>> {
        let tables = self.resolutions.read().expect(POISONED);
        let Some(table) = tables.get(dim_id) else {
            return Ok(None);
        };
        table
            .clone()
            .downcast::<ResolutionTable<N>>()
            .map(Some)
            .map_err(|_| {
                MetaStorageError::Internal("resolution table registered with another arity")
            })
    }

    pub(super) fn clear_footprint(&self) {
        *self.footprint.write().expect(POISONED) = None;
        self.executions.write().expect(POISONED).clear();
    }

    pub(super) fn get_footprint(&self) -> Option<RunFootprint> {
        self.footprint.read().expect(POISONED).clone()
    }

    pub(super) fn upsert_run(&self, footprint: &RunFootprint) {
        *self.footprint.write().expect(POISONED) = Some(footprint.clone());
    }

    pub(super) fn put_execution(&self, run_id: Uuid, execution_id: Uuid) {
        self.executions.write().expect(POISONED).insert(
            execution_id,
            Execution {
                run_id,
                ended_at: None,
                end_reason: None,
            },
        );
    }

    pub(super) fn update_execution_on_finish(&self, footprint: &RunFootprint, execution_id: Uuid) {
        let mut executions = self.executions.write().expect(POISONED);
        let Some(execution) = executions.get_mut(&execution_id) else {
            return;
        };
        if execution.run_id != footprint.metadata.run_id {
            return;
        }
        execution.ended_at = Some(footprint.at);
        execution.end_reason = Some(footprint.metadata.state.to_string());
    }
}
