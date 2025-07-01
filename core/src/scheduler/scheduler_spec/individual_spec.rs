use std::{pin::Pin, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    meta_storage::{MetaClient, MetaStorage},
    operon::RunningState,
    scheduler::{
        ControlEventReceiver, IndividualRebuilder, PeerEventReceiver, PeerEventSenderMap,
        SchedulerError,
    },
    service::OperonService,
    storage::OperonStorage,
    ui::UiState,
};

#[async_trait]
pub trait IndividualSpec<Svc, Sto>: Send + Sync + 'static
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

    /// Run a check on the data consistency between the data storage and the metadata storage.
    /// Return `true` if the data storage holds all needed data to restore, `false` if it does not.
    async fn check_consistency(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
    ) -> Result<bool, SchedulerError>; // `Scheduler::check_consistency`, 5611~

    async fn get_status(&self, client: MetaClient<'_>) -> Result<(i64, i64, i64), SchedulerError>;

    async fn prepare_rebuild(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
    ) -> Result<Box<dyn IndividualRebuilder>, SchedulerError>; // `Scheduler::run` 6049~

    // Implement `start` (`IndividualScheduler::run` 4462~) and call it using different initial data_fetching

    #[allow(clippy::too_many_arguments)]
    fn start_clean(
        &self,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: MetaStorage,
        ui_state: Arc<RwLock<UiState>>,
        peer_txs: PeerEventSenderMap<Svc::JobEnum, Svc::ResolutionEnum>,
        peer_rx: PeerEventReceiver<Svc::JobEnum, Svc::ResolutionEnum>,
        ctrl_rx: ControlEventReceiver,
    ) -> Pin<Box<dyn Future<Output = RunningState> + Send + 'static>>; // call `start` with empty Vector (`Scheduler::run` 6023)

    #[allow(clippy::too_many_arguments)]
    fn start_rebuild(
        &self,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: MetaStorage,
        ui_state: Arc<RwLock<UiState>>,
        peer_txs: PeerEventSenderMap<Svc::JobEnum, Svc::ResolutionEnum>,
        peer_rx: PeerEventReceiver<Svc::JobEnum, Svc::ResolutionEnum>,
        ctrl_rx: ControlEventReceiver,
    ) -> Pin<Box<dyn Future<Output = RunningState> + Send + 'static>>; // fetch `get_all_queued` and then get call `start` (`Scheduler::run` 6302)

    #[allow(clippy::too_many_arguments)]
    fn start_restore(
        &self,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: MetaStorage,
        ui_state: Arc<RwLock<UiState>>,
        peer_txs: PeerEventSenderMap<Svc::JobEnum, Svc::ResolutionEnum>,
        peer_rx: PeerEventReceiver<Svc::JobEnum, Svc::ResolutionEnum>,
        ctrl_rx: ControlEventReceiver,
    ) -> Pin<Box<dyn Future<Output = RunningState> + Send + 'static>>; // fetch `get_all_queued` and then get call `start` (`Scheduler::run` 6302), possibly merge with `start_rebuild`
}
