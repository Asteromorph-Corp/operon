use async_trait::async_trait;

use crate::meta_storage::MetaClient;
use crate::scheduler::SchedulerError;

// TODO: Temporary hard-coded value, may be added to the `OperonOptions` in the future.
/// How many `(job, resolution)` chains a rebuilder keeps in flight on the rebuild transaction.
pub const REBUILD_CONCURRENCY: usize = 64;

#[async_trait]
pub trait JobRebuilder: Send + Sync + 'static {
    async fn rebuild(&self, client: MetaClient<'_>) -> Result<(), SchedulerError>; // `Scheduler::run` 6179~
}
