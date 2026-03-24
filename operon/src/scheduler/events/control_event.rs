pub type ControlEventReceiver = tokio::sync::watch::Receiver<ControlEvent>;
pub type ControlEventSender = tokio::sync::watch::Sender<ControlEvent>;
pub type ControlEventError = tokio::sync::watch::error::SendError<ControlEvent>;

/// `IndividualScheduler`-UI communication events.
///
/// These are used for communication between individual schedulers and the UI.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum ControlEvent {
    /// Start the scheduler, finding out the recovery state.
    #[default]
    Start,
    /// Perform a check on the consistency between the storages.
    Check {
        mode: crate::ui::CheckMode,
    },
    /// Perform a run.
    Run {
        fresh: bool,
        rebuild: bool,
    },

    /// Pause executing new jobs.
    /// Note that pausing the scheduler does not stop ongoing jobs,
    /// neither does it stop handling events (i.e. updating the ticket storage).
    Pause {
        targets: Vec<String>,
        cascade: bool,
    },
    /// Resume executing new jobs.
    Resume {
        targets: Vec<String>,
    },
    Quit {
        force: bool,
        no_exit: bool,
    },
    Exit,
}
