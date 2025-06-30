use std::{collections::HashMap, sync::Arc};

use tokio::{sync::RwLock, task::JoinSet};

use crate::{
    meta_storage::MetaStorage,
    operon::RunningState,
    scheduler::{ControlEventReceiver, IndividualSpec, PeerEventReceiver, PeerEventSender},
    service::OperonService,
    storage::OperonStorage,
    ui::UiState,
};

/// Helper struct for `Scheduler::prepare_channel`
///
/// An association of `JobManager` and event receiver channel
pub(crate) struct SpecWithRx<'a, Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub schedule: &'a dyn IndividualSpec<Svc, Sto>,
    pub peer_rx: PeerEventReceiver,
}

impl<'a, Svc, Sto> SpecWithRx<'a, Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn new(schedule: &'a dyn IndividualSpec<Svc, Sto>, peer_rx: PeerEventReceiver) -> Self {
        Self { schedule, peer_rx }
    }
}

/// Helper struct for `Scheduler::prepare_channel`
pub(crate) struct SpecsWithChannels<'a, Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub spec_with_rx: Vec<SpecWithRx<'a, Svc, Sto>>,
    pub peer_txs: HashMap<&'static str, PeerEventSender>,
}

impl<'a, Svc, Sto> SpecsWithChannels<'a, Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn new(
        schedules_with_rx: Vec<SpecWithRx<'a, Svc, Sto>>,
        peer_txs: HashMap<&'static str, PeerEventSender>,
    ) -> Self {
        Self {
            spec_with_rx: schedules_with_rx,
            peer_txs,
        }
    }
}

impl<'a, Svc, Sto> SpecsWithChannels<'a, Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn start_clean(
        self,
        service: &Arc<Svc>,
        storage: &Arc<Sto>,
        meta_storage: &MetaStorage,
        ui_state: &Arc<RwLock<UiState>>,
        ctrl_rx: &ControlEventReceiver,
    ) -> (
        JoinSet<RunningState>,
        HashMap<&'static str, PeerEventSender>,
    ) {
        let handles = JoinSet::from_iter(self.spec_with_rx.into_iter().map(
            |SpecWithRx { schedule, peer_rx }| {
                schedule.start_rebuild(
                    service.clone(),
                    storage.clone(),
                    meta_storage.clone(),
                    ui_state.clone(),
                    self.peer_txs.clone(),
                    peer_rx,
                    ctrl_rx.clone(),
                )
            },
        ));
        (handles, self.peer_txs)
    }

    pub fn start_rebuild(
        self,
        service: &Arc<Svc>,
        storage: &Arc<Sto>,
        meta_storage: &MetaStorage,
        ui_state: &Arc<RwLock<UiState>>,
        ctrl_rx: &ControlEventReceiver,
    ) -> JoinSet<RunningState> {
        JoinSet::from_iter(
            self.spec_with_rx
                .into_iter()
                .map(|SpecWithRx { schedule, peer_rx }| {
                    schedule.start_rebuild(
                        service.clone(),
                        storage.clone(),
                        meta_storage.clone(),
                        ui_state.clone(),
                        self.peer_txs.clone(),
                        peer_rx,
                        ctrl_rx.clone(),
                    )
                }),
        )
    }

    pub fn start_restore(
        self,
        service: &Arc<Svc>,
        storage: &Arc<Sto>,
        meta_storage: &MetaStorage,
        ui_state: &Arc<RwLock<UiState>>,
        ctrl_rx: &ControlEventReceiver,
    ) -> JoinSet<RunningState> {
        JoinSet::from_iter(
            self.spec_with_rx
                .into_iter()
                .map(|SpecWithRx { schedule, peer_rx }| {
                    schedule.start_restore(
                        service.clone(),
                        storage.clone(),
                        meta_storage.clone(),
                        ui_state.clone(),
                        self.peer_txs.clone(),
                        peer_rx,
                        ctrl_rx.clone(),
                    )
                }),
        )
    }
}
