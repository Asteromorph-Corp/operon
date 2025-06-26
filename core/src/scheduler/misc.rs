use std::collections::HashMap;

use crate::{
    meta_storage::MetaStorage,
    scheduler::{IndividualSchedule, PeerEventReceiver, PeerEventSender},
    service::OperonService,
    storage::OperonStorage,
};

/// Helper struct for `Scheduler::prepare_channel`
///
/// An association of `JobManager` and event receiver channel
pub(super) struct ScheduleWithRx<'a, Sto, Svc, MSto>
where
    Sto: OperonStorage,
    Svc: OperonService,
    MSto: MetaStorage,
{
    pub(super) schedule: &'a dyn IndividualSchedule<Sto, Svc, MSto>,
    pub(super) peer_rx: PeerEventReceiver,
}

impl<'a, Sto, Svc, MSto> ScheduleWithRx<'a, Sto, Svc, MSto>
where
    Sto: OperonStorage,
    Svc: OperonService,
    MSto: MetaStorage,
{
    pub(super) fn new(
        schedule: &'a dyn IndividualSchedule<Sto, Svc, MSto>,
        peer_rx: PeerEventReceiver,
    ) -> Self {
        Self { schedule, peer_rx }
    }
}

/// Helper struct for `Scheduler::prepare_channel`
pub(super) struct PreparedSchedules<'a, Sto, Svc, MSto>
where
    Sto: OperonStorage,
    Svc: OperonService,
    MSto: MetaStorage,
{
    pub(super) schedules_with_rx: Vec<ScheduleWithRx<'a, Sto, Svc, MSto>>,
    pub(super) peer_txs: HashMap<String, PeerEventSender>,
}

impl<'a, Sto, Svc, MSto> PreparedSchedules<'a, Sto, Svc, MSto>
where
    Sto: OperonStorage,
    Svc: OperonService,
    MSto: MetaStorage,
{
    pub(super) fn new(
        schedules_with_rx: Vec<ScheduleWithRx<'a, Sto, Svc, MSto>>,
        peer_txs: HashMap<String, PeerEventSender>,
    ) -> Self {
        Self {
            schedules_with_rx,
            peer_txs,
        }
    }
}
