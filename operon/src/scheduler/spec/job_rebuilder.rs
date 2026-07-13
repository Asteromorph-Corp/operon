use async_trait::async_trait;

use crate::meta_storage::MetaBackend;
use crate::scheduler::SchedulerError;

#[async_trait]
pub trait JobRebuilder<MSto: MetaBackend>: Send + Sync + 'static {
    async fn rebuild(&self, client: MSto::Client<'_>) -> Result<(), SchedulerError>;
}
