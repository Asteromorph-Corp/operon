use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::meta_storage::{MetaClient, MetaStorage};
use crate::operon::RunningState;
use crate::scheduler::{
    ControlEventReceiver, IndividualScheduler, JobRebuilder, JobSpec, SchedulerError,
    ServicePeerEventReceiver, ServicePeerEventSenderMap, SpecWithMetadata,
};
use crate::schema::{Job, Ticket};
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::UiState;

#[async_trait]
/// An helper trait to expose `Resolution`, `Ticket`, and `JobManager` interfaces while being dyn
/// compatible.
///
/// The trait is automatically implemented for any structs that implements `JobManagerImpl`,
/// and is used to initialize the job metadata storage and to run the individual job schedulers.
pub trait JobHandler<Svc, Sto>: Send + Sync + 'static
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    fn job_id(&self) -> &'static str;

    fn pool_size(&self) -> usize {
        1 // Default pool size, can be overridden by the job configuration
    }

    /// Initialize the PSQL fact storage for the primary resolution.
    ///
    /// This function should be idempotent,
    /// i.e. calling it multiple times, or calling it on an already-initialized storage should do
    /// nothing.
    async fn init_resolution(&self, client: MetaClient<'_>) -> Result<(), SchedulerError>;

    /// Clear the primary resolution from the PSQL fact storage, assuming the table is already
    /// initialized.
    async fn clear_resolution(&self, client: MetaClient<'_>) -> Result<(), SchedulerError>; // `facts_psql::clear`, 877~

    /// Initialize the PSQL ticket storage.
    ///
    /// This function should be idempotent,
    /// i.e. calling it multiple times, or calling it on an already-initialized storage should do
    /// nothing.
    async fn init_tickets(&self, client: MetaClient<'_>) -> Result<(), SchedulerError>; // `tickets_psql::init`, 1079~

    /// Clear the data from the PSQL ticket storage, assuming the tables are already initialized.
    async fn clear_tickets(&self, client: MetaClient<'_>) -> Result<(), SchedulerError>; // `tickets_psql::clear`, 1623~

    /// Put the default (fully unresolved) tickets into the PSQL ticket storage.
    async fn put_default_tickets(&self, client: MetaClient<'_>) -> Result<(), SchedulerError>;

    /// Get the status of the tickets.
    async fn get_status(&self, client: MetaClient<'_>) -> Result<(i64, i64, i64), SchedulerError>;

    /// Run a check on the data consistency between the data storage and the metadata storage.
    /// Return `true` if the data storage holds all needed data to restore, `false` if it does not.
    async fn check_consistency(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
    ) -> Result<bool, SchedulerError>; // `Scheduler::check_consistency`, 5611~

    /// Prepare the job rebuilder for the given storage and metadata client by fetching the
    /// necessary data.
    async fn prepare_rebuild(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
    ) -> Result<Box<dyn JobRebuilder>, SchedulerError>; // `Scheduler::run` 6049~

    /// Start the job manager, initializing the individual scheduler and running it.
    #[allow(clippy::too_many_arguments)]
    fn run_scheduler(
        &self,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: MetaStorage,
        ui_state: Arc<RwLock<UiState>>,
        peer_txs: ServicePeerEventSenderMap<Svc>,
        peer_rx: ServicePeerEventReceiver<Svc>,
        ctrl_rx: ControlEventReceiver,
        clean: bool,
    ) -> Pin<Box<dyn Future<Output = RunningState> + Send + 'static>>; // call `start` with empty Vector (`Scheduler::run` 6023)
}

#[async_trait]
impl<Svc, Sto, JS, const N: usize> JobHandler<Svc, Sto> for SpecWithMetadata<Svc, Sto, JS, N>
where
    Svc: OperonService,
    Sto: OperonStorage,
    JS: JobSpec<Svc, Sto, Job = Job<N>, Ticket = Ticket<N>> + Clone,
{
    fn job_id(&self) -> &'static str {
        self.job_meta.id
    }

    fn pool_size(&self) -> usize {
        self.spec.pool_size()
    }

    async fn init_resolution(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        if let Some(spawn_dim_meta) = self.job_meta.spawn_dim_meta() {
            client.resolution(spawn_dim_meta).init().await?;
        }
        Ok(())
    }

    async fn clear_resolution(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        if let Some(spawn_dim_meta) = self.job_meta.spawn_dim_meta() {
            client.resolution(spawn_dim_meta).clear().await?;
        }
        Ok(())
    }

    async fn init_tickets(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        client.ticket(self.job_meta).init().await?;
        Ok(())
    }

    async fn clear_tickets(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        client.ticket(self.job_meta).clear().await?;
        Ok(())
    }

    async fn put_default_tickets(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        let default_ticket = self.spec.default_ticket();
        client.ticket(self.job_meta).put(default_ticket).await?;
        Ok(())
    }

    async fn get_status(&self, client: MetaClient<'_>) -> Result<(i64, i64, i64), SchedulerError> {
        let status = client.ticket(self.job_meta).get_status().await?;
        Ok(status)
    }

    async fn check_consistency(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
    ) -> Result<bool, SchedulerError> {
        self.spec.check_consistency(storage, client).await
    }

    async fn prepare_rebuild(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
    ) -> Result<Box<dyn JobRebuilder>, SchedulerError> {
        self.spec.prepare_rebuild(storage, client).await
    }

    #[allow(clippy::too_many_arguments)]
    fn run_scheduler(
        &self,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: MetaStorage,
        ui_state: Arc<RwLock<UiState>>,
        peer_txs: ServicePeerEventSenderMap<Svc>,
        peer_rx: ServicePeerEventReceiver<Svc>,
        ctrl_rx: ControlEventReceiver,
        clean: bool,
    ) -> Pin<Box<dyn Future<Output = RunningState> + Send + 'static>> {
        let individual_scheduler = IndividualScheduler::new(
            self.clone(),
            service,
            storage,
            meta_storage,
            self.pool_size(),
            ui_state,
        );
        Box::pin(individual_scheduler.run(peer_txs, peer_rx, ctrl_rx, clean))
    }
}
