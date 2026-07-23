use std::sync::Arc;

use crate::meta_storage::MetaBackend;
use crate::meta_storage::mem::error::MemResult;
use crate::meta_storage::mem::store::MemStore;
use crate::meta_storage::mem::{
    MemClient, MemConn, MemMetaError, MemResolutionQueryBuilder, MemTicketQueryBuilder, MemTx,
};

/// The in-memory implementation of the metadata backend.
///
/// Holds the whole run's metadata in process, sharing one store across every clone of the handle.
///
/// # Durability
///
/// The store is volatile and non-transactional: it has nothing to lock against a second Operon
/// instance, and a failure part-way through a write leaves the partial write in place. It suits
/// work that is rebuilt from scratch on failure.
#[derive(Clone, Default)]
pub struct MemMetaStorage {
    store: Arc<MemStore>,
}

impl std::fmt::Debug for MemMetaStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemMetaStorage").finish_non_exhaustive()
    }
}

impl MetaBackend for MemMetaStorage {
    type Error = MemMetaError;
    type Conn<'a> = MemConn;
    type Tx<'a> = MemTx<'a>;
    type Client<'a> = MemClient<'a>;
    type Ticket<'a, const N: usize> = MemTicketQueryBuilder<'a, N>;
    type Resolution<'a, const N: usize> = MemResolutionQueryBuilder<'a, N>;

    async fn worker_conn(&self) -> MemResult<MemConn> {
        Ok(MemConn::new(self.store.clone()))
    }

    async fn scheduler_conn(&self) -> MemResult<MemConn> {
        Ok(MemConn::new(self.store.clone()))
    }

    /// Acquires the exclusive lock guarding this run's metadata: a no-op.
    ///
    /// The store is process-local and reachable only through this instance, so no second Operon
    /// instance can share it.
    async fn ensure_lock(&self) -> MemResult<()> {
        Ok(())
    }

    /// Re-checks the lock taken by [`ensure_lock`](Self::ensure_lock): a no-op.
    async fn check_lock(&self) -> MemResult<()> {
        Ok(())
    }
}
