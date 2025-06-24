use std::collections::HashMap;

use crate::{
    dimension::{Job, Resolution},
    meta_storage::MetaStorage,
    scheduler::{JobManager, PeerEventReceiver, PeerEventSender},
    service::OperonService,
    storage::OperonStorage,
};

/// Helper struct for `Scheduler::prepare_channel`
///
/// An association of `JobManager` and event receiver channel
pub(super) struct JobManagerWithRx<'a, Sto, Svc, MSto, J, R>
where
    Sto: OperonStorage,
    Svc: OperonService,
    MSto: MetaStorage,
    J: Job,
    R: Resolution,
{
    pub(super) manager: &'a dyn JobManager<Sto, Svc, MSto, J, R>,
    pub(super) peer_rx: PeerEventReceiver<J, R>,
}

impl<'a, Sto, Svc, MSto, J, R> JobManagerWithRx<'a, Sto, Svc, MSto, J, R>
where
    Sto: OperonStorage,
    Svc: OperonService,
    MSto: MetaStorage,
    J: Job,
    R: Resolution,
{
    pub(super) fn new(
        manager: &'a dyn JobManager<Sto, Svc, MSto, J, R>,
        peer_rx: PeerEventReceiver<J, R>,
    ) -> Self {
        Self { manager, peer_rx }
    }
}

/// Helper struct for `Scheduler::prepare_channel`
pub(super) struct PreparedJobs<'a, Sto, Svc, MSto, J, R>
where
    Sto: OperonStorage,
    Svc: OperonService,
    MSto: MetaStorage,
    J: Job,
    R: Resolution,
{
    pub(super) managers_with_rx: Vec<JobManagerWithRx<'a, Sto, Svc, MSto, J, R>>,
    pub(super) peer_txs: HashMap<String, PeerEventSender<J, R>>,
}

impl<'a, Sto, Svc, MSto, J, R> PreparedJobs<'a, Sto, Svc, MSto, J, R>
where
    Sto: OperonStorage,
    Svc: OperonService,
    MSto: MetaStorage,
    J: Job,
    R: Resolution,
{
    pub(super) fn new(
        managers_with_rx: Vec<JobManagerWithRx<'a, Sto, Svc, MSto, J, R>>,
        peer_txs: HashMap<String, PeerEventSender<J, R>>,
    ) -> Self {
        Self {
            managers_with_rx,
            peer_txs,
        }
    }
}
