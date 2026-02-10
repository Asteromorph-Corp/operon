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
    Check { mode: crate::ui::CheckMode },
    /// Perform a clean run.
    CleanRun,
    /// Perform a rebuilding run from an `AbortedChecked` state.
    RebuildRun,
    /// Perform a restoring run from a `GracefullyStopped` state.
    RestoreRun,
    /// Pause executing new jobs.
    /// Note that pausing the scheduler does not stop ongoing jobs,
    /// neither does it stop handling events (i.e. updating the ticket storage).
    Pause { targets: Vec<String>, cascade: bool },
    /// Resume executing new jobs.
    Resume { targets: Vec<String> },
    /// Gracefully stop the scheduler:
    /// * Stop executing new jobs (= pause),
    /// * wait for all ongoing jobs to finish,
    /// * handle all pending events,
    /// * place a persistent indication that the scheduler stopped gracefully,
    /// * and then exit.
    ///
    /// Effectively, "pause everything and wait long enough".
    GracefulStop,
    /// Abort the scheduler immediately.
    Abort,
}

impl ControlEvent {
    pub fn pause(targets: Vec<String>, cascade: bool) -> Self {
        ControlEvent::Pause { targets, cascade }
    }

    pub fn resume(targets: Vec<String>) -> Self {
        ControlEvent::Resume { targets }
    }
}
