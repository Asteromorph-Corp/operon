use async_trait::async_trait;

use crate::meta_storage::MetaClient;
use crate::scheduler::{JobRebuilder, PeerEventSenders, SchedulerError};
use crate::schema_base::{Job, Resolution, Ticket};
use crate::service::OperonService;
use crate::storage::OperonStorage;

#[async_trait]
pub trait JobSpec<Svc, Sto>: Clone + Send + Sync + 'static
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    type Job: Job;
    type Resolution: Resolution;
    type Ticket: Ticket<Job = Self::Job, Resolution = Self::Resolution>;
    type PeerEventSenders: PeerEventSenders<Svc::JobEnum, Svc::ResolutionEnum>;

    fn pool_size(&self) -> usize;

    /// Run a check on the data consistency between the data storage and the metadata storage.
    /// Return `true` if the data storage holds all needed data to restore, `false` if it does not.
    async fn check_consistency(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
    ) -> Result<bool, SchedulerError>;

    /// Prepare the job rebuilder for the given storage and metadata client by fetching the
    /// necessary data.
    async fn prepare_rebuild(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
    ) -> Result<Box<dyn JobRebuilder>, SchedulerError>;

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

    async fn send_on_finish(
        &self,
        peer_txs: &Self::PeerEventSenders,
        job: Self::Job,
        resolution: Self::Resolution,
    ) -> Result<(), SchedulerError>;

    async fn on_receive_job(
        &self,
        client: MetaClient<'_>,
        job: Svc::JobEnum,
    ) -> Result<Vec<Self::Ticket>, SchedulerError>;

    async fn on_receive_resolution(
        &self,
        client: MetaClient<'_>,
        resolution: Svc::ResolutionEnum,
    ) -> Result<Vec<Self::Ticket>, SchedulerError>;
}
