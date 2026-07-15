use async_trait::async_trait;

use crate::meta_storage::MetaBackend;
use crate::scheduler::SchedulerError;

// TODO: Temporary hard-coded value, may be added to the `OperonOptions` in the future.
/// How many `(job, resolution)` chains a rebuilder keeps in flight on the rebuild transaction.
pub const REBUILD_CONCURRENCY: usize = 64;

#[async_trait]
pub trait JobRebuilder<MSto: MetaBackend>: Send + Sync + 'static {
    async fn rebuild(&self, client: MSto::Client<'_>) -> Result<(), SchedulerError<MSto::Error>>;
}
