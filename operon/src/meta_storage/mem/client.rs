use std::sync::Arc;

use uuid::Uuid;

use crate::meta_storage::mem::error::MemResult;
use crate::meta_storage::mem::store::MemStore;
use crate::meta_storage::mem::{MemMetaStorage, MemResolutionQueryBuilder, MemTicketQueryBuilder};
use crate::meta_storage::{MetaClientApi, MetaConnApi, MetaTxApi};
use crate::schema::{DimensionMetadata, JobMetadata, RunFootprint};

/// A handle on the in-memory store.
///
/// The store is process-local, so a "connection" is just a shared handle to it. It owns its handle
/// rather than borrowing the backend, so it can be handed out for `'static`.
pub struct MemConn {
    store: Arc<MemStore>,
}

impl MemConn {
    pub(super) fn new(store: Arc<MemStore>) -> Self {
        Self { store }
    }
}

impl MetaConnApi<MemMetaStorage> for MemConn {
    /// Opens a transaction.
    ///
    /// The in-memory store is non-transactional: writes land immediately and are never rolled
    /// back.
    async fn transaction(&mut self) -> MemResult<MemTx<'_>> {
        Ok(MemTx { store: &self.store })
    }

    fn as_client(&self) -> MemClient<'_> {
        MemClient(&self.store)
    }
}

/// A transaction over the in-memory store.
pub struct MemTx<'a> {
    store: &'a MemStore,
}

impl MetaTxApi<MemMetaStorage> for MemTx<'_> {
    /// Commits the transaction: a no-op, as its writes already landed.
    async fn commit(self) -> MemResult<()> {
        Ok(())
    }

    /// Rolls the transaction back: a no-op, as its writes already landed.
    async fn rollback(self) -> MemResult<()> {
        Ok(())
    }

    fn as_client(&self) -> MemClient<'_> {
        MemClient(self.store)
    }
}

/// A borrowed handle for issuing queries against the in-memory store.
#[derive(Clone, Copy)]
pub struct MemClient<'a>(&'a MemStore);

impl MetaClientApi<MemMetaStorage> for MemClient<'_> {
    fn ticket<const N: usize>(&self, job_meta: JobMetadata<N>) -> MemTicketQueryBuilder<'_, N> {
        self.0.ticket(job_meta)
    }

    fn resolution<const N: usize>(
        &self,
        dim_meta: DimensionMetadata<N>,
    ) -> MemResolutionQueryBuilder<'_, N> {
        self.0.resolution(dim_meta)
    }

    // --- Schema initialization ---
    //
    // The in-memory store has no schema: tables are registered on first use by the ticket and
    // resolution builders, and statuses and summaries are held natively.

    async fn init_schema(&self) -> MemResult<()> {
        Ok(())
    }

    async fn init_ticket_hash(&self) -> MemResult<()> {
        Ok(())
    }

    async fn init_dimension_hash(&self) -> MemResult<()> {
        Ok(())
    }

    async fn init_ticket_status_type(&self) -> MemResult<()> {
        Ok(())
    }

    async fn init_ticket_summary(&self) -> MemResult<()> {
        Ok(())
    }

    // --- Footprint ---

    async fn init_footprint(&self) -> MemResult<()> {
        Ok(())
    }

    async fn clear_footprint(&self) -> MemResult<()> {
        self.0.clear_footprint();
        Ok(())
    }

    async fn get_footprint(&self) -> MemResult<Option<RunFootprint>> {
        Ok(self.0.get_footprint())
    }

    async fn upsert_run(&self, footprint: &RunFootprint) -> MemResult<()> {
        self.0.upsert_run(footprint);
        Ok(())
    }

    async fn put_execution(&self, run_id: Uuid, execution_id: Uuid) -> MemResult<()> {
        self.0.put_execution(run_id, execution_id);
        Ok(())
    }

    async fn update_execution_on_finish(
        &self,
        footprint: &RunFootprint,
        execution_id: Uuid,
    ) -> MemResult<()> {
        self.0.update_execution_on_finish(footprint, execution_id);
        Ok(())
    }
}
