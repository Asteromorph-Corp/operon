use async_trait::async_trait;

use crate::scheduler::SchedulerError;

#[async_trait]
pub trait JobRebuilder: Send + Sync + 'static {
    async fn explode(&self, primary_resolution: usize) -> Result<(), SchedulerError>; // `Scheduler::run` 6167~6177, look at explode_* functions

    async fn rebuild(&self) -> Result<(), SchedulerError>; // `Scheduler::run` 6179~
}
