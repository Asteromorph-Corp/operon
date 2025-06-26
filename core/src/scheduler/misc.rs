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
pub(super) struct ScheduleWithRx<'a, Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaStorage,
{
    pub(super) schedule: &'a dyn IndividualSchedule<Svc, Sto, MSto>,
    pub(super) peer_rx: PeerEventReceiver,
}

impl<'a, Svc, Sto, MSto> ScheduleWithRx<'a, Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaStorage,
{
    pub(super) fn new(
        schedule: &'a dyn IndividualSchedule<Svc, Sto, MSto>,
        peer_rx: PeerEventReceiver,
    ) -> Self {
        Self { schedule, peer_rx }
    }
}

/// Helper struct for `Scheduler::prepare_channel`
pub(super) struct PreparedSchedules<'a, Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaStorage,
{
    pub(super) schedules_with_rx: Vec<ScheduleWithRx<'a, Svc, Sto, MSto>>,
    pub(super) peer_txs: HashMap<&'static str, PeerEventSender>,
}

impl<'a, Svc, Sto, MSto> PreparedSchedules<'a, Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaStorage,
{
    pub(super) fn new(
        schedules_with_rx: Vec<ScheduleWithRx<'a, Svc, Sto, MSto>>,
        peer_txs: HashMap<&'static str, PeerEventSender>,
    ) -> Self {
        Self {
            schedules_with_rx,
            peer_txs,
        }
    }
}
