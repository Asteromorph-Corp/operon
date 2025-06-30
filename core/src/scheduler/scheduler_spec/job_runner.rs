use async_trait::async_trait;

use crate::{
    meta_storage::MetaClient,
    misc::{Job, Resolution, Ticket},
    scheduler::{PeerEventSenders, SchedulerError},
    service::OperonService,
    storage::OperonStorage,
};

#[async_trait]
pub trait JobRunner<Svc, Sto>: Clone + Send + Sync + 'static
where
    Sto: OperonStorage,
    Svc: OperonService,
{
    type Job: Job;
    type Resolution: Resolution;
    type Ticket: Ticket<Job = Self::Job, Resolution = Self::Resolution>;
    type PeerEventSenders: PeerEventSenders;

    /// The identifier for this job type.
    fn job_type() -> &'static str;

    /// Check if this job type is a descendant of the given job type.
    fn is_descendant_of(other: &str) -> bool;

    /// Call the user function and stores the result in the storage.
    ///
    /// Return the dimension resolution that was resolved by this job, if any.
    async fn run_job(
        &self,
        service: &Svc,
        storage: &Sto,
        client: MetaClient<'_>,
        job: &Self::Job,
    ) -> Result<Self::Resolution, SchedulerError>;

    async fn mark_done(
        &self,
        client: MetaClient<'_>,
        job: &Self::Job,
    ) -> Result<(), SchedulerError>;

    /// Put a resolution into the storage.
    async fn put_resolution(
        &self,
        client: MetaClient<'_>,
        resolution: &Self::Resolution,
    ) -> Result<(), SchedulerError>;
}
