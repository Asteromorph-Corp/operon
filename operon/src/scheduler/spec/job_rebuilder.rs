use async_trait::async_trait;

use crate::meta_storage::MetaClient;
use crate::scheduler::SchedulerError;

#[async_trait]
pub trait JobRebuilder: Send + Sync + 'static {
    async fn rebuild(&self, client: MetaClient<'_>) -> Result<(), SchedulerError>; // `Scheduler::run` 6179~
}
