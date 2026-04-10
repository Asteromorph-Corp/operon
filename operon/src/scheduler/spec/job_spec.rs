use async_trait::async_trait;

use crate::meta_storage::MetaClient;
use crate::scheduler::events::PeerEventSenders;
use crate::scheduler::{JobRebuilder, SchedulerError};
use crate::schema::{
    CheckMode, JobLike, JobMetadata, ResolutionLike, SharedProgress, TicketExplosion, TicketLike,
};
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub struct SpecWithMetadata<Svc, Sto, JS, const N: usize>
where
    Svc: OperonService,
    Sto: OperonStorage,
    JS: JobSpec<Svc, Sto>,
{
    pub spec: JS,
    pub job_meta: JobMetadata<N>,
    _phantom: std::marker::PhantomData<(Svc, Sto)>,
}

impl<Svc, Sto, JS, const N: usize> SpecWithMetadata<Svc, Sto, JS, N>
where
    Svc: OperonService,
    Sto: OperonStorage,
    JS: JobSpec<Svc, Sto>,
{
    pub fn new(spec: JS, job_meta: JobMetadata<N>) -> Self {
        Self {
            spec,
            job_meta,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<Svc, Sto, JS, const N: usize> Clone for SpecWithMetadata<Svc, Sto, JS, N>
where
    Svc: OperonService,
    Sto: OperonStorage,
    JS: JobSpec<Svc, Sto> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            spec: self.spec.clone(),
            job_meta: self.job_meta,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
pub trait JobSpec<Svc, Sto>: Clone + Send + Sync + 'static
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    type Job: JobLike;
    type Resolution: ResolutionLike;
    type Ticket: TicketLike;
    type PeerEventSenders: PeerEventSenders<Svc::JobEnum, Svc::ResolutionEnum, Svc::TicketEnum>;

    fn all_upstream_jobs(&self) -> Vec<&'static str>;
    fn pool_size(&self) -> usize;

    fn default_ticket(&self) -> Self::Ticket;

    /// Run a check on the data consistency between the data storage and the metadata storage.
    /// Return `true` if the data storage holds all needed data to restore, `false` if it does not.
    async fn check_consistency(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
        mode: CheckMode,
    ) -> Result<bool, SchedulerError>;

    /// Prepare the job rebuilder for the given storage and metadata client by fetching the
    /// necessary data.
    async fn prepare_rebuild(
        &self,
        storage: &Sto,
        progress: SharedProgress,
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
        job: Self::Job,
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
        peer_txs: &Self::PeerEventSenders,
        resolution: Svc::ResolutionEnum,
    ) -> Result<Vec<Self::Ticket>, SchedulerError>;

    async fn on_receive_explosion(
        &self,
        client: MetaClient<'_>,
        explosion: TicketExplosion<Svc::TicketEnum>,
    ) -> Result<Vec<Self::Ticket>, SchedulerError>;
}
