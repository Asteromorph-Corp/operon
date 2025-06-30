use async_trait::async_trait;

use crate::{
    meta_storage::MetaClient,
    misc::{Job, Resolution, Ticket},
    scheduler::{IndividualSpec, PeerEventSenders, SchedulerError},
    service::OperonService,
    storage::OperonStorage,
};

#[async_trait]
pub trait JobManager<Svc, Sto>: IndividualSpec<Svc, Sto> + Clone + Send + Sync + 'static
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    type Job: Job;
    type Resolution: Resolution;
    type Ticket: Ticket<Job = Self::Job, Resolution = Self::Resolution>;
    type PeerEventSenders: PeerEventSenders<Svc::JobEnum, Svc::ResolutionEnum>;

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

    async fn send_event(
        &self,
        peer_txs: &Self::PeerEventSenders,
        job: Self::Job,
        resolution: Self::Resolution,
    ) -> Result<(), SchedulerError>;

    async fn on_job_ready_tickets(
        &self,
        client: MetaClient<'_>,
        job: Svc::JobEnum,
    ) -> Result<Vec<Self::Ticket>, SchedulerError>;

    async fn on_resolution_ready_tickets(
        &self,
        client: MetaClient<'_>,
        resolution: Svc::ResolutionEnum,
    ) -> Result<Vec<Self::Ticket>, SchedulerError>;
}
