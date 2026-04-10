pub type IndividualControlEventReceiver = tokio::sync::mpsc::Receiver<IndividualControlEvent>;
pub type IndividualControlEventSender = tokio::sync::mpsc::Sender<IndividualControlEvent>;

/// `IndividualScheduler`-`Scheduler` communication events.
///
/// These events are produced when `Scheduler` handles
/// [`ControlEvent`](crate::scheduler::events::IndividualControlEvent), where appropriate events are
/// multiplexed to the
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndividualControlEvent {
    Pause,
    /// Resume executing new jobs.
    Resume,
    Quit {
        force: bool,
    },
}
