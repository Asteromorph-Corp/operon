use async_trait::async_trait;

use crate::{meta_storage::MetaClient, scheduler::SchedulerError};

#[async_trait]
pub trait IndividualRebuilder: Send + Sync + 'static {
    async fn explode(
        &self,
        client: MetaClient<'_>,
        primary_ub: usize,
    ) -> Result<(), SchedulerError>; // `Scheduler::run` 6167~6177, look at explode_* functions

    async fn rebuild(&self, client: MetaClient<'_>) -> Result<(), SchedulerError>; // `Scheduler::run` 6179~
}
