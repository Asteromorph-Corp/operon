use std::collections::HashSet;

use crate::schema::CheckMode;

pub type ControlEventReceiver = tokio::sync::mpsc::Receiver<ControlEvent>;
pub type ControlEventSender = tokio::sync::mpsc::Sender<ControlEvent>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEventInner {
    Unspecified {
        redo_inconsistent_tasks: bool,
    },
    Fresh,
    Rebuild {
        skip: HashSet<String>,
        redo_inconsistent_tasks: bool,
    },
}

/// `IndividualScheduler`-UI communication events.
///
/// These are used for communication between individual schedulers and the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlEvent {
    /// Perform a check on the consistency between the storages.
    Check {
        mode: CheckMode,
    },
    /// Perform a run.
    Run(RunEventInner),
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

impl ControlEvent {
    pub const FRESH_RUN: Self = Self::Run(RunEventInner::Fresh);

    pub const FORCE_QUIT: Self = Self::Quit {
        force: true,
        no_exit: false,
    };
}
