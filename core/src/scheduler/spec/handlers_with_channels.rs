use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::task::JoinSet;

use crate::meta_storage::MetaStorage;
use crate::operon::RunningState;
use crate::scheduler::{ControlEventReceiver, JobHandler, PeerEventReceiver, PeerEventSenderMap};
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::UiState;

/// Helper struct for `Scheduler::prepare_channel`
///
/// An association of `JobManager` and event receiver channel
pub(crate) struct HandlerWithRx<'a, Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub handler: &'a dyn JobHandler<Svc, Sto>,
    pub peer_rx: PeerEventReceiver<Svc::JobEnum, Svc::ResolutionEnum>,
}

impl<'a, Svc, Sto> HandlerWithRx<'a, Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn new(
        handler: &'a dyn JobHandler<Svc, Sto>,
        peer_rx: PeerEventReceiver<Svc::JobEnum, Svc::ResolutionEnum>,
    ) -> Self {
        Self { handler, peer_rx }
    }
}

/// Helper struct for `Scheduler::prepare_channel`
pub(crate) struct HandlersWithChannels<'a, Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub handlers_with_rx: Vec<HandlerWithRx<'a, Svc, Sto>>,
    pub peer_txs: PeerEventSenderMap<Svc::JobEnum, Svc::ResolutionEnum>,
}

impl<'a, Svc, Sto> HandlersWithChannels<'a, Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn new(
        handlers_with_rx: Vec<HandlerWithRx<'a, Svc, Sto>>,
        peer_txs: PeerEventSenderMap<Svc::JobEnum, Svc::ResolutionEnum>,
    ) -> Self {
        Self {
            handlers_with_rx,
            peer_txs,
        }
    }
}

impl<'a, Svc, Sto> HandlersWithChannels<'a, Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn run_schedulers(
        self,
        service: &Arc<Svc>,
        storage: &Arc<Sto>,
        meta_storage: &MetaStorage,
        ui_state: &Arc<RwLock<UiState>>,
        ctrl_rx: &ControlEventReceiver,
        clean: bool,
    ) -> (
        JoinSet<RunningState>,
        PeerEventSenderMap<Svc::JobEnum, Svc::ResolutionEnum>,
    ) {
        let handles = JoinSet::from_iter(self.handlers_with_rx.into_iter().map(
            |HandlerWithRx { handler, peer_rx }| {
                handler.run_scheduler(
                    service.clone(),
                    storage.clone(),
                    meta_storage.clone(),
                    ui_state.clone(),
                    self.peer_txs.clone(),
                    peer_rx,
                    ctrl_rx.clone(),
                    clean,
                )
            },
        ));
        (handles, self.peer_txs)
    }
}
