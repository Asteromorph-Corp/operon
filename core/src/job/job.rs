use async_trait::async_trait;

use crate::scheduler::SchedulerError;

#[async_trait]
pub trait Job {
    async fn check_consistency(&self, primary_ub: usize) -> Result<bool, SchedulerError>; // `Scheduler::check_consistency`, 5611~

    async fn get_initial_data(&self, primary_ub: usize) -> Result<Vec<u8>, SchedulerError>; // `Scheduler::run` let initial_data = .., 5996~
}
