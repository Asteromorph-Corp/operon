use std::collections::HashMap;

use crate::scheduler::SchedulerError;

/// `IndividualScheduler`-`IndividualScheduler` communication events.
///
/// These are used for communication between individual schedulers,
/// where each scheduler should modify its tickets based on the events.\
#[derive(Debug, Clone)]
pub struct PeerEvent;

#[derive(Debug, Clone)]
pub enum PeerEventSender {
    Up(tokio::sync::mpsc::Sender<PeerEvent>),
    Downgraded(tokio::sync::mpsc::WeakSender<PeerEvent>),
}

pub type PeerEventReceiver = tokio::sync::mpsc::Receiver<PeerEvent>;

impl PeerEventSender {
    pub async fn send(&self) -> Result<(), SchedulerError> {
        match self {
            PeerEventSender::Up(tx) => tx
                .send(PeerEvent)
                .await
                .map_err(|_| SchedulerError::PeerEventSendFailed),
            PeerEventSender::Downgraded(_) => Err(SchedulerError::SendThroughDowngradedSender),
        }
    }

    pub fn downgrade(&mut self) {
        match self {
            PeerEventSender::Up(tx) => {
                let weak = tx.downgrade();
                *self = PeerEventSender::Downgraded(weak);
            }
            PeerEventSender::Downgraded(_) => {}
        }
    }
}

pub trait PeerEventSenders {
    /// Constructs a new `PeerEventSenders` instance from the given senders.
    ///
    /// Remove the senders from the map.
    ///
    /// TODO: tx channels probably close when dropped, so taking owned `HashMap` should work. But I don't want to break anything, will refactor later.
    fn gather_from(senders: &mut HashMap<&'static str, PeerEventSender>) -> Self;

    fn downgrade_all(&mut self);
}

impl PeerEventSenders for () {
    fn gather_from(_: &mut HashMap<&'static str, PeerEventSender>) -> Self {}

    fn downgrade_all(&mut self) {}
}
