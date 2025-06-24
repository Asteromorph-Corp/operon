use crate::scheduler::SchedulerError;

/// `IndividualScheduler`-`IndividualScheduler` communication events.
///
/// These are used for communication between individual schedulers,
/// where each scheduler should modify its tickets based on the events.
#[derive(Debug, Clone)]
pub enum PeerEvent<J, R> {
    /// A job was run and finished.
    Job(J),
    /// A resolution was made known.
    Resolution(R),
}

#[derive(Debug, Clone)]
pub enum PeerEventSender<J, R> {
    Up(::tokio::sync::mpsc::Sender<PeerEvent<J, R>>),
    Downgraded(::tokio::sync::mpsc::WeakSender<PeerEvent<J, R>>),
}
pub type PeerEventReceiver<Job, Resolution> =
    ::tokio::sync::mpsc::Receiver<PeerEvent<Job, Resolution>>;

impl<J, R> PeerEventSender<J, R> {
    pub async fn send(&self, event: PeerEvent<J, R>) -> Result<(), SchedulerError> {
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
