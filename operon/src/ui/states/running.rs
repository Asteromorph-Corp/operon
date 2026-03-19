use crate::scheduler::{ControlEvent, ControlEventSender, ExecutionState};
use crate::ui::states::quitting::QuittingState;
use crate::ui::states::{NextState, UiState};
use crate::ui::{Command, ProgressMap, UiError};

/// The state of the ui loop while jobs are running.
pub struct RunningState {
    /// The [`ExecutionState`] for all tasks.
    exec_states: ExecutionStates,
    /// Whether all individual schedulers have exited.
    schedulers_exited: bool,
    ctrl_tx: ControlEventSender,
}

impl RunningState {
    pub fn new(ctrl_tx: ControlEventSender) -> Self {
        Self {
            exec_states: ExecutionStates(Vec::new()),
            schedulers_exited: false,
            ctrl_tx,
        }
    }

    pub fn boxed(ctrl_tx: ControlEventSender) -> Box<Self> {
        Box::new(Self::new(ctrl_tx))
    }
}

impl UiState for RunningState {
    fn handle_progress(mut self: Box<Self>, progress: ProgressMap) -> Result<NextState, UiError> {
        self.schedulers_exited = progress.values().all(|p| p.4);
        self.exec_states = ExecutionStates(progress.values().map(|p| p.3).collect());
        Ok(NextState::Next(self))
    }

    fn handle_command(self: Box<Self>, command: Command) -> Result<NextState, UiError> {
        match command {
            Command::Run { .. } => log::warn!(
                "Already run. Use `exit` or `quit` to terminate the current session before starting a new run."
            ),
            Command::Check { .. } => log::warn!("Cannot check after the run has already started."),
            Command::Quit { no_exit: false, .. } if self.schedulers_exited => {
                log::warn!("Nothing to quit.")
            }
            Command::Quit { .. } if self.schedulers_exited => return Ok(NextState::Exit),
            Command::Quit { force, no_exit } => {
                let (force, exit) = match (force, self.exec_states.overall_state()) {
                    (true, ExecutionState::Running | ExecutionState::Paused) => {
                        log::warn!("Sent abort request, stopping immediately...");
                        (true, !no_exit)
                    }
                    (false, ExecutionState::Running | ExecutionState::Paused) => {
                        log::info!("Sent stop request, stopping gracefully...");
                        (false, !no_exit)
                    }
                    (false, ExecutionState::Error) => {
                        log::warn!(
                            "Cannot gracefully stop due to previous errors. Defaulting to a force quit."
                        );
                        (true, false)
                    }
                    _ => (true, !no_exit),
                };

                let event = if force {
                    ControlEvent::Abort
                } else {
                    ControlEvent::GracefulStop
                };
                self.ctrl_tx.send(event)?;
                return Ok(NextState::Next(QuittingState::boxed(
                    force,
                    exit,
                    self.ctrl_tx,
                )));
            }
            Command::Pause { .. } if !self.exec_states.any_running() => {
                log::warn!("No running jobs to pause, did you mean to `exit` or `quit` instead?")
            }
            Command::Pause { targets, cascade } => {
                self.ctrl_tx.send(ControlEvent::pause(targets, cascade))?
            }
            Command::Resume { .. } if !self.exec_states.any_paused() => {
                log::warn!("No paused jobs to resume.")
            }
            Command::Resume { targets } => self.ctrl_tx.send(ControlEvent::resume(targets))?,
            Command::Exit if self.schedulers_exited => return Ok(NextState::Exit),
            Command::Exit => log::warn!(
                "Cannot exit while jobs are running or paused. \
                Use `quit` for a graceful stop, or `quit --force` to abort all jobs."
            ),
            Command::Clear => {}
            Command::Help => {}
        }

        Ok(NextState::Next(self))
    }
}

struct ExecutionStates(Vec<ExecutionState>);

impl ExecutionStates {
    pub fn overall_state(&self) -> ExecutionState {
        if self.0.contains(&ExecutionState::Error) {
            ExecutionState::Error
        } else if self.0.iter().all(|s| *s == ExecutionState::Finished) {
            ExecutionState::Finished
        } else if self.0.iter().all(|s| *s == ExecutionState::Paused) {
            ExecutionState::Paused
        } else if self
            .0
            .iter()
            .all(|s| *s == ExecutionState::Stopped || *s == ExecutionState::Finished)
        {
            ExecutionState::Stopped
        } else {
            ExecutionState::Running
        }
    }

    pub fn any_running(&self) -> bool {
        self.0.contains(&ExecutionState::Running)
    }

    pub fn any_paused(&self) -> bool {
        self.0.contains(&ExecutionState::Paused)
    }
}
