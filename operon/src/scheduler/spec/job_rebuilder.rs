use async_trait::async_trait;

use crate::meta_storage::MetaBackend;
use crate::scheduler::SchedulerError;
use crate::service::OperonService;
use crate::storage::OperonStorage;

// TODO: Temporary hard-coded value, may be added to the `OperonOptions` in the future.
/// How many `(job, resolution)` chains a rebuilder keeps in flight on the rebuild transaction.
pub const REBUILD_CONCURRENCY: usize = 64;

/// One task's replay of a previous run, over the data that run left in the entity storage.
///
/// [`JobSpec::prepare_rebuild`](crate::scheduler::JobSpec::prepare_rebuild) reads the entities the
/// task already produced and hands back a rebuilder over them, which then writes the metadata
/// those entities imply.
#[async_trait]
pub trait JobRebuilder<Svc: OperonService, Sto: OperonStorage, MSto: MetaBackend>:
    Send + Sync + 'static
{
    /// Replays every job this task already completed: marking their tickets done, recording the
    /// dimensions they resolved, and propagating both to the downstream tasks.
    ///
    /// Jobs whose data survived without a queued ticket to match are left out, so the rebuild
    /// restores only what the metadata still accounts for.
    async fn rebuild(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;
}
