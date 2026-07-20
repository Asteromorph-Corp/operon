use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;

use crate::meta_storage::{MetaBackend, MetaClientApi, MetaResolutionApi, MetaTicketApi};
use crate::scheduler::events::{
    IndividualControlEventReceiver, ServicePeerEventReceiver, ServicePeerEventSenderMap,
};
use crate::scheduler::individual_scheduler::IndividualScheduler;
use crate::scheduler::spec::SpecWithMetadata;
use crate::scheduler::{JobRebuilder, JobSpec, SchedulerError};
use crate::schema::{CheckMode, Job, SharedProgress, Ticket};
use crate::service::OperonService;
use crate::storage::OperonStorage;

#[async_trait]
/// An helper trait to expose `Resolution`, `Ticket`, and `JobManager` interfaces while being dyn
/// compatible.
///
/// The trait is automatically implemented for any structs that implements `JobManagerImpl`,
/// and is used to initialize the job metadata storage and to run the individual job schedulers.
pub trait JobHandler<Svc, Sto, MSto>: Send + Sync + 'static
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    fn job_id(&self) -> &'static str;

    fn all_upstream_jobs(&self) -> Vec<&'static str>;

    fn pool_size(&self) -> usize {
        1 // Default pool size, can be overridden by the job configuration
    }

    /// Initialize the fact storage for the primary resolution.
    ///
    /// This function should be idempotent,
    /// i.e. calling it multiple times, or calling it on an already-initialized storage should do
    /// nothing.
    async fn init_resolution(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<MSto::Error, Sto::Error, Svc::Error>>;

    /// Clear the primary resolution from the fact storage, assuming the table is already
    /// initialized.
    async fn clear_resolution(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<MSto::Error, Sto::Error, Svc::Error>>;

    /// Initialize the ticket storage.
    ///
    /// This function should be idempotent,
    /// i.e. calling it multiple times, or calling it on an already-initialized storage should do
    /// nothing.
    async fn init_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<MSto::Error, Sto::Error, Svc::Error>>;

    /// Clear the data from the ticket storage, assuming the tables are already initialized.
    async fn clear_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<MSto::Error, Sto::Error, Svc::Error>>;

    /// Put the default (fully unresolved) tickets into the ticket storage.
    async fn put_default_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<MSto::Error, Sto::Error, Svc::Error>>;

    /// Get the status of the tickets.
    async fn get_status(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(i64, i64, i64), SchedulerError<MSto::Error, Sto::Error, Svc::Error>>;

    /// Run a check on the data consistency between the data storage and the metadata storage.
    /// Return `true` if the data storage holds all needed data to restore, `false` if it does not.
    async fn check_consistency(
        &self,
        storage: &Sto,
        client: MSto::Client<'_>,
        mode: CheckMode,
    ) -> Result<bool, SchedulerError<MSto::Error, Sto::Error, Svc::Error>>;

    /// Prepare the job rebuilder for the given storage and metadata client by fetching the
    /// necessary data.
    async fn prepare_rebuild(
        &self,
        storage: &Sto,
        progress: SharedProgress,
        client: MSto::Client<'_>,
    ) -> Result<
        Box<dyn JobRebuilder<Svc, Sto, MSto>>,
        SchedulerError<MSto::Error, Sto::Error, Svc::Error>,
    >;

    /// Start the job manager, initializing the individual scheduler and running it.
    #[allow(clippy::too_many_arguments)]
    fn run_scheduler(
        &self,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: MSto,
        progress: SharedProgress,
        peer_txs: ServicePeerEventSenderMap<Svc>,
        peer_rx: ServicePeerEventReceiver<Svc>,
        ctrl_rx: IndividualControlEventReceiver,
        clean: bool,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
}

#[async_trait]
impl<Svc, Sto, JS, MSto, const N: usize> JobHandler<Svc, Sto, MSto>
    for SpecWithMetadata<Svc, Sto, JS, N>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
    JS: JobSpec<Svc, Sto, MSto, Job = Job<N>, Ticket = Ticket<N>> + Clone,
{
    fn job_id(&self) -> &'static str {
        self.job_meta.id
    }

    fn all_upstream_jobs(&self) -> Vec<&'static str> {
        self.spec.all_upstream_jobs()
    }

    fn pool_size(&self) -> usize {
        self.spec.pool_size()
    }

    async fn init_resolution(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<MSto::Error, Sto::Error, Svc::Error>> {
        if let Some(spawn_dim_meta) = self.job_meta.spawn_dim_meta() {
            client.resolution(spawn_dim_meta).init().await?;
        }
        Ok(())
    }

    async fn clear_resolution(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<MSto::Error, Sto::Error, Svc::Error>> {
        if let Some(spawn_dim_meta) = self.job_meta.spawn_dim_meta() {
            client.resolution(spawn_dim_meta).clear().await?;
        }
        Ok(())
    }

    async fn init_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<MSto::Error, Sto::Error, Svc::Error>> {
        client.ticket(self.job_meta).init().await?;
        Ok(())
    }

    async fn clear_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<MSto::Error, Sto::Error, Svc::Error>> {
        client.ticket(self.job_meta).clear().await?;
        Ok(())
    }

    async fn put_default_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<MSto::Error, Sto::Error, Svc::Error>> {
        let default_ticket = self.spec.default_ticket();
        client.ticket(self.job_meta).put(default_ticket).await?;
        Ok(())
    }

    async fn get_status(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(i64, i64, i64), SchedulerError<MSto::Error, Sto::Error, Svc::Error>> {
        let status = client.ticket(self.job_meta).get_status().await?;
        Ok(status)
    }

    async fn check_consistency(
        &self,
        storage: &Sto,
        client: MSto::Client<'_>,
        mode: CheckMode,
    ) -> Result<bool, SchedulerError<MSto::Error, Sto::Error, Svc::Error>> {
        self.spec.check_consistency(storage, client, mode).await
    }

    async fn prepare_rebuild(
        &self,
        storage: &Sto,
        progress: SharedProgress,
        client: MSto::Client<'_>,
    ) -> Result<
        Box<dyn JobRebuilder<Svc, Sto, MSto>>,
        SchedulerError<MSto::Error, Sto::Error, Svc::Error>,
    > {
        self.spec.prepare_rebuild(storage, progress, client).await
    }

    #[allow(clippy::too_many_arguments)]
    fn run_scheduler(
        &self,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: MSto,
        progress: SharedProgress,
        peer_txs: ServicePeerEventSenderMap<Svc>,
        peer_rx: ServicePeerEventReceiver<Svc>,
        ctrl_rx: IndividualControlEventReceiver,
        clean: bool,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
        let individual_scheduler = IndividualScheduler::new(
            self.clone(),
            service,
            storage,
            meta_storage,
            self.pool_size(),
            progress,
        );
        Box::pin(individual_scheduler.run(peer_txs, peer_rx, ctrl_rx, clean))
    }
}
