use std::collections::HashMap;
use std::fmt::Debug;

use async_trait::async_trait;

use crate::scheduler::SchedulerError;
use crate::schema::{JobEnum, ResolutionEnum};

/// `IndividualScheduler`-`IndividualScheduler` communication events.
///
/// These are used for communication between individual schedulers,
/// where each scheduler should modify its tickets based on the events.
#[derive(Debug, Clone)]
pub enum PeerEvent<JE: JobEnum, RE: ResolutionEnum> {
    Job(JE),
    Resolution(RE),
    Explosion(RE),
}

#[derive(Debug, Clone)]
pub enum PeerEventSender<JE: JobEnum, RE: ResolutionEnum> {
    Up(tokio::sync::mpsc::Sender<PeerEvent<JE, RE>>),
    Downgraded(tokio::sync::mpsc::WeakSender<PeerEvent<JE, RE>>),
}

pub type PeerEventReceiver<JE, RE> = tokio::sync::mpsc::Receiver<PeerEvent<JE, RE>>;
pub type PeerEventSenderMap<JE, RE> = HashMap<&'static str, PeerEventSender<JE, RE>>;

impl<JE: JobEnum, RE: ResolutionEnum> PeerEventSender<JE, RE> {
    pub async fn send(&self, event: PeerEvent<JE, RE>) -> Result<(), SchedulerError> {
        match self {
            PeerEventSender::Up(tx) => tx
                .send(event)
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

#[async_trait]
pub trait PeerEventSenders<JE: JobEnum, RE: ResolutionEnum>: Send + Sync {
    /// Constructs a new `PeerEventSenders` instance from the given senders.
    ///
    /// Remove the senders from the map.
    fn gather_from(senders: HashMap<&'static str, PeerEventSender<JE, RE>>) -> Self;

    fn downgrade_all(&mut self);
}

#[async_trait]
impl<JE: JobEnum, RE: ResolutionEnum> PeerEventSenders<JE, RE> for () {
    fn gather_from(_: HashMap<&'static str, PeerEventSender<JE, RE>>) -> Self {}

    fn downgrade_all(&mut self) {}
}
