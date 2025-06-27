use std::{collections::HashMap, pin::Pin, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    meta_storage::{MetaClient, MetaStorage},
    operon::RunningState,
    scheduler::{
        ControlEventReceiver, JobRebuilder, PeerEventReceiver, PeerEventSender, SchedulerError,
    },
    service::OperonService,
    storage::OperonStorage,
    ui::UiState,
};

#[async_trait]
pub trait IndividualSchedule<Svc, Sto, MSto>: Send + Sync + 'static
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaStorage,
{
    fn id(&self) -> &'static str;

    async fn check_consistency(
        &self,
        client: MetaClient<'_>,
        schema_prefix: &str,
        primary_ub: usize,
    ) -> Result<bool, SchedulerError>; // `Scheduler::check_consistency`, 5611~

    async fn prepare_rebuild(
        &self,
        tx: MetaClient<'_>,
    ) -> Result<Box<dyn JobRebuilder>, SchedulerError>; // `Scheduler::run` 6049~

    // Implement `start` (`IndividualScheduler::run` 4462~) and call it using different initial data_fetching

    #[allow(clippy::too_many_arguments)]
    fn start_clean(
        &self,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: Arc<MSto>,
        ui_state: Arc<RwLock<UiState>>,
        peer_txs: HashMap<&'static str, PeerEventSender>,
        peer_rx: PeerEventReceiver,
        ctrl_rx: ControlEventReceiver,
    ) -> Pin<Box<dyn Future<Output = RunningState> + Send + 'static>>; // call `start` with empty Vector (`Scheduler::run` 6023)

    #[allow(clippy::too_many_arguments)]
    fn start_rebuild(
        &self,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: Arc<MSto>,
        ui_state: Arc<RwLock<UiState>>,
        peer_txs: HashMap<&'static str, PeerEventSender>,
        peer_rx: PeerEventReceiver,
        ctrl_rx: ControlEventReceiver,
    ) -> Pin<Box<dyn Future<Output = RunningState> + Send + 'static>>; // fetch `get_all_queued` and then get call `start` (`Scheduler::run` 6302)

    #[allow(clippy::too_many_arguments)]
    fn start_restore(
        &self,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: Arc<MSto>,
        ui_state: Arc<RwLock<UiState>>,
        peer_txs: HashMap<&'static str, PeerEventSender>,
        peer_rx: PeerEventReceiver,
        ctrl_rx: ControlEventReceiver,
    ) -> Pin<Box<dyn Future<Output = RunningState> + Send + 'static>>; // fetch `get_all_queued` and then get call `start` (`Scheduler::run` 6302), possibly merge with `start_rebuild`
}
