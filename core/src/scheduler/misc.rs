use std::collections::HashMap;

use crate::{
    meta_storage::MetaStorage,
    scheduler::{JobManager, PeerEventReceiver, PeerEventSender},
    service::OperonService,
    storage::OperonStorage,
};

/// Helper struct for `Scheduler::prepare_channel`
///
/// An association of `JobManager` and event receiver channel
pub(super) struct JobManagerWithRx<'a, Sto, Svc, MSto>
where
    Sto: OperonStorage,
    Svc: OperonService,
    MSto: MetaStorage,
{
    pub(super) manager: &'a dyn JobManager<Sto, Svc, MSto>,
    pub(super) peer_rx: PeerEventReceiver<MSto::JobEnum, MSto::ResolutionEnum>,
}

impl<'a, Sto, Svc, MSto> JobManagerWithRx<'a, Sto, Svc, MSto>
where
    Sto: OperonStorage,
    Svc: OperonService,
    MSto: MetaStorage,
{
    pub(super) fn new(
        manager: &'a dyn JobManager<Sto, Svc, MSto>,
        peer_rx: PeerEventReceiver<MSto::JobEnum, MSto::ResolutionEnum>,
    ) -> Self {
        Self { manager, peer_rx }
    }
}

/// Helper struct for `Scheduler::prepare_channel`
pub(super) struct PreparedJobs<'a, Sto, Svc, MSto>
where
    Sto: OperonStorage,
    Svc: OperonService,
    MSto: MetaStorage,
{
    pub(super) managers_with_rx: Vec<JobManagerWithRx<'a, Sto, Svc, MSto>>,
    pub(super) peer_txs: HashMap<String, PeerEventSender<MSto::JobEnum, MSto::ResolutionEnum>>,
}

impl<'a, Sto, Svc, MSto> PreparedJobs<'a, Sto, Svc, MSto>
where
    Sto: OperonStorage,
    Svc: OperonService,
    MSto: MetaStorage,
{
    pub(super) fn new(
        managers_with_rx: Vec<JobManagerWithRx<'a, Sto, Svc, MSto>>,
        peer_txs: HashMap<String, PeerEventSender<MSto::JobEnum, MSto::ResolutionEnum>>,
    ) -> Self {
        Self {
            managers_with_rx,
            peer_txs,
        }
    }
}
