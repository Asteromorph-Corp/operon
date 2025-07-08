use std::{pin::Pin, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    meta_storage::{MetaClient, MetaStorage},
    operon::RunningState,
    scheduler::{
        ControlEventReceiver, IndividualScheduler, JobRebuilder, JobSpec, PeerEventReceiver,
        PeerEventSenderMap, SchedulerError,
    },
    schema_base::{JobSql, ResolutionSql, TicketSql},
    service::OperonService,
    storage::OperonStorage,
    ui::UiState,
};

#[async_trait]
/// An helper trait to expose `Resolution`, `Ticket`, and `JobManager` interfaces while being dyn compatible.
///
/// The trait is automatically implemented for any structs that implements `JobManagerImpl`,
/// and is used to initialize the job metadata storage and to run the individual job schedulers.
pub trait JobHandler<Svc, Sto>: Send + Sync + 'static
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    fn job_id(&self) -> &'static str;

    /// Initialize the PSQL fact storage for the primary resolution.
    ///
    /// This function should be idempotent,
    /// i.e. calling it multiple times, or calling it on an already-initialized storage should do nothing.
    async fn init_resolution(&self, client: MetaClient<'_>) -> Result<(), SchedulerError>;

    /// Clear the primary resolution from the PSQL fact storage, assuming the table is already initialized.
    async fn clear_resolution(&self, client: MetaClient<'_>) -> Result<(), SchedulerError>; // `facts_psql::clear`, 877~

    /// Initialize the PSQL ticket storage.
    ///
    /// This function should be idempotent,
    /// i.e. calling it multiple times, or calling it on an already-initialized storage should do nothing.
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

    /// Prepare the job rebuilder for the given storage and metadata client by fetching the necessary data.
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
        peer_txs: PeerEventSenderMap<Svc::JobEnum, Svc::ResolutionEnum>,
        peer_rx: PeerEventReceiver<Svc::JobEnum, Svc::ResolutionEnum>,
        ctrl_rx: ControlEventReceiver,
        clean: bool,
    ) -> Pin<Box<dyn Future<Output = RunningState> + Send + 'static>>; // call `start` with empty Vector (`Scheduler::run` 6023)
}

#[async_trait]
impl<Svc, Sto, J, R, T, JS> JobHandler<Svc, Sto> for JS
where
    Svc: OperonService,
    Sto: OperonStorage,
    J: JobSql,
    R: ResolutionSql,
    T: TicketSql<Job = J, Resolution = R>,
    JS: JobSpec<Svc, Sto, Job = J, Resolution = R, Ticket = T> + Clone,
{
    fn job_id(&self) -> &'static str {
        J::id()
    }

    async fn init_resolution(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        R::init_table(client).await?;
        Ok(())
    }

    async fn clear_resolution(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        R::clear_table(client).await?;
        Ok(())
    }

    async fn init_tickets(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        T::init_table(client).await?;
        Ok(())
    }

    async fn clear_tickets(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        T::clear_table(client).await?;
        Ok(())
    }

    async fn put_default_tickets(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        T::new().put(client).await?;
        Ok(())
    }

    async fn get_status(&self, client: MetaClient<'_>) -> Result<(i64, i64, i64), SchedulerError> {
        let status = T::get_status(client).await?;
        Ok(status)
    }

    async fn check_consistency(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
    ) -> Result<bool, SchedulerError> {
        JobSpec::check_consistency(self, storage, client).await
    }

    async fn prepare_rebuild(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
    ) -> Result<Box<dyn JobRebuilder>, SchedulerError> {
        JobSpec::prepare_rebuild(self, storage, client).await
    }

    #[allow(clippy::too_many_arguments)]
    fn run_scheduler(
        &self,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: MetaStorage,
        ui_state: Arc<RwLock<UiState>>,
        peer_txs: PeerEventSenderMap<Svc::JobEnum, Svc::ResolutionEnum>,
        peer_rx: PeerEventReceiver<Svc::JobEnum, Svc::ResolutionEnum>,
        ctrl_rx: ControlEventReceiver,
        clean: bool,
    ) -> Pin<Box<dyn Future<Output = RunningState> + Send + 'static>> {
        let individual_scheduler =
            IndividualScheduler::new(self.clone(), service, storage, meta_storage, 8, ui_state);
        Box::pin(individual_scheduler.run(peer_txs, peer_rx, ctrl_rx, clean))
    }
}
