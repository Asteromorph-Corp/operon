use async_trait::async_trait;

use crate::{
    dimension::{Job, Resolution},
    meta_storage::MetaClient,
    scheduler::SchedulerError,
    service::OperonService,
    storage::OperonStorage,
};

#[async_trait]
pub trait JobRunner<Svc, Sto, J, R>: Send + Sync + 'static
where
    Sto: OperonStorage,
    Svc: OperonService,
    J: Job,
    R: Resolution,
{
    fn clone_box(&self) -> Box<dyn JobRunner<Svc, Sto, J, R>>;

    /// Call the user function and stores the result in the storage.
    ///
    /// Return the dimension resolution that was resolved by this job, if any.
    async fn run_job(
        &self,
        conn: MetaClient<'_>,
        service: &Svc,
        storage: &Sto,
        job: &J,
    ) -> Result<R, SchedulerError>;

    async fn mark_done(&self, conn: MetaClient<'_>, job: &J) -> Result<(), SchedulerError>;

    /// Put a resolution into the storage.
    async fn put_resolution(
        &self,
        conn: MetaClient<'_>,
        resolution: &R,
    ) -> Result<(), SchedulerError>;
}
