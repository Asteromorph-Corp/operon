use std::collections::HashMap;
use std::fmt::Debug;

use async_trait::async_trait;

use crate::schema::{JobEnum, ResolutionEnum, TicketEnum, TicketExplosion};
use crate::service::OperonService;

/// `IndividualScheduler`-`IndividualScheduler` communication events.
///
/// These are used for communication between individual schedulers,
/// where each scheduler should modify its tickets based on the events.
#[derive(Debug, Clone)]
pub enum PeerEvent<J: JobEnum, R: ResolutionEnum, T: TicketEnum> {
    Job(J),
    Resolution(R),
    Explosion(TicketExplosion<T>),
}

pub type PeerEventSender<J, R, T> = tokio::sync::mpsc::Sender<PeerEvent<J, R, T>>;
pub(crate) type PeerEventReceiver<J, R, T> = tokio::sync::mpsc::Receiver<PeerEvent<J, R, T>>;
pub type PeerEventSenderMap<J, R, T> = HashMap<&'static str, PeerEventSender<J, R, T>>;

pub(crate) type ServicePeerEventReceiver<Svc> = PeerEventReceiver<
    <Svc as OperonService>::JobEnum,
    <Svc as OperonService>::ResolutionEnum,
    <Svc as OperonService>::TicketEnum,
>;
pub(crate) type ServicePeerEventSenderMap<Svc> = PeerEventSenderMap<
    <Svc as OperonService>::JobEnum,
    <Svc as OperonService>::ResolutionEnum,
    <Svc as OperonService>::TicketEnum,
>;

#[async_trait]
pub trait PeerEventSenders<J: JobEnum, R: ResolutionEnum, T: TicketEnum>: Send + Sync {
    /// Constructs a new `PeerEventSenders` instance from the given senders.
    ///
    /// Remove the senders from the map.
    fn gather_from(senders: PeerEventSenderMap<J, R, T>) -> Self;
}

#[async_trait]
impl<J: JobEnum, R: ResolutionEnum, T: TicketEnum> PeerEventSenders<J, R, T> for () {
    fn gather_from(_: PeerEventSenderMap<J, R, T>) -> Self {}
}
