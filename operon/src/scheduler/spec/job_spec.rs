use async_trait::async_trait;

use crate::meta_storage::MetaBackend;
use crate::scheduler::events::PeerEventSenders;
use crate::scheduler::{JobRebuilder, SchedulerError};
use crate::schema::{
    CheckMode, JobLike, JobMetadata, ResolutionLike, SharedProgress, TicketExplosion, TicketLike,
};
use crate::service::OperonService;
use crate::storage::OperonStorage;

/// One task's [`JobSpec`] paired with the metadata describing the task it was generated for.
pub struct SpecWithMetadata<Svc, Sto, JS, const N: usize> {
    pub spec: JS,
    pub job_meta: JobMetadata<N>,
    _phantom: std::marker::PhantomData<(Svc, Sto)>,
}

impl<Svc, Sto, JS, const N: usize> SpecWithMetadata<Svc, Sto, JS, N> {
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
    JS: Clone,
{
    fn clone(&self) -> Self {
        Self {
            spec: self.spec.clone(),
            job_meta: self.job_meta,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Everything the scheduler needs to run one task of a pipeline.
///
/// [`define_operon!`](crate::define_operon) generates an implementation per task, which is what
/// ties the generic scheduler to a specific pipeline: the dimensions a task iterates over, the
/// upstream tasks it waits for, and the downstream tasks it announces its jobs to.
#[async_trait]
pub trait JobSpec<Svc, Sto, MSto>: Clone + Send + Sync + 'static
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    /// One execution of this task, addressed by its coordinate.
    type Job: JobLike;

    /// The extent this task's spawned dimension resolves to, or `()` when it spawns none.
    type Resolution: ResolutionLike;

    /// The record tracking one of this task's jobs towards being runnable.
    type Ticket: TicketLike;

    /// The channels this task announces its results over, one per downstream task.
    type PeerEventSenders: PeerEventSenders<Svc::JobEnum, Svc::ResolutionEnum, Svc::TicketEnum>;

    /// The ids of every task this one transitively depends on, in lexicographic order.
    fn all_upstream_jobs(&self) -> Vec<&'static str>;

    /// How many of this task's jobs may run at once.
    fn pool_size(&self) -> usize;

    /// The ticket this task starts from, its coordinate wholly unresolved and its quota set to the
    /// number of upstream jobs a job of this task waits for.
    fn default_ticket(&self) -> Self::Ticket;

    /// Run a check on the data consistency between the data storage and the metadata storage.
    /// Return `true` if the data storage holds all needed data to restore, `false` if it does not.
    async fn check_consistency(
        &self,
        storage: &Sto,
        client: MSto::Client<'_>,
        mode: CheckMode,
    ) -> Result<bool, SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;

    /// Prepare this task's [`JobRebuilder`] for the given storage and metadata client by fetching
    /// the necessary data.
    async fn prepare_rebuild(
        &self,
        storage: &Sto,
        progress: SharedProgress,
        client: MSto::Client<'_>,
    ) -> Result<
        Box<dyn JobRebuilder<Svc, Sto, MSto>>,
        SchedulerError<Svc::Error, Sto::Error, MSto::Error>,
    >;

    /// Call the user function and stores the result in the storage.
    ///
    /// Return the dimension resolution that was resolved by this job, if any.
    async fn run_job(
        &self,
        service: &Svc,
        storage: &Sto,
        meta: MSto,
        job: Self::Job,
    ) -> Result<Self::Resolution, SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;

    /// Announces a finished job, and the dimension it resolved, to every downstream task that
    /// consumes them.
    ///
    /// A closed peer channel is not an error: it means that scheduler has already exited.
    async fn send_on_finish(
        &self,
        peer_txs: &Self::PeerEventSenders,
        job: Self::Job,
        resolution: Self::Resolution,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;

    /// Counts an upstream job's completion against this task's tickets, and returns the ones it
    /// made runnable.
    async fn on_receive_job(
        &self,
        client: MSto::Client<'_>,
        job: Svc::JobEnum,
    ) -> Result<Vec<Self::Ticket>, SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;

    /// Explodes this task's tickets along the resolved dimension, and returns the ones that
    /// became runnable.
    ///
    /// Each exploded ticket is forwarded to the downstream tasks that aggregate over that
    /// dimension, since their own tickets now wait on more jobs than their quota accounts for.
    async fn on_receive_resolution(
        &self,
        client: MSto::Client<'_>,
        peer_txs: &Self::PeerEventSenders,
        resolution: Svc::ResolutionEnum,
    ) -> Result<Vec<Self::Ticket>, SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;

    /// Takes an explosion forwarded by an upstream task, raising the quota of the tickets that
    /// aggregate over the exploded dimension, and returns the ones it made runnable.
    async fn on_receive_explosion(
        &self,
        client: MSto::Client<'_>,
        explosion: TicketExplosion<Svc::TicketEnum>,
    ) -> Result<Vec<Self::Ticket>, SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;
}
