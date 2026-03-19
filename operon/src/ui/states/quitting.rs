use crate::scheduler::{ControlEvent, ControlEventSender};
use crate::ui::states::{NextState, UiState};
use crate::ui::{Command, ProgressMap, UiError};

/// The state of the ui loop while handling `quit` command.
pub struct QuittingState {
    force: bool,
    exit: bool,
    /// Whether all individual schedulers have exited.
    schedulers_exited: bool,
    ctrl_tx: ControlEventSender,
}

impl QuittingState {
    pub fn new(force: bool, exit: bool, ctrl_tx: ControlEventSender) -> Self {
        Self {
            force,
            exit,
            schedulers_exited: false,
            ctrl_tx,
        }
    }

    pub fn boxed(force: bool, exit: bool, ctrl_tx: ControlEventSender) -> Box<Self> {
        Box::new(Self::new(force, exit, ctrl_tx))
    }
}

impl UiState for QuittingState {
    fn handle_progress(mut self: Box<Self>, progress: ProgressMap) -> Result<NextState, UiError> {
        self.schedulers_exited = progress.values().all(|p| p.4);
        if self.exit && self.schedulers_exited {
            return Ok(NextState::Exit);
        }
        Ok(NextState::Next(self))
    }

    fn handle_command(mut self: Box<Self>, command: Command) -> Result<NextState, UiError> {
        match command {
            Command::Run { .. } => log::warn!(
                "Already run. Use `exit` or `quit` to terminate the current session before starting a new run."
            ),
            Command::Check { .. } => log::warn!("Cannot check after the run has already started."),
            Command::Quit { .. } if self.force => {
                log::warn!("Already processing an abort.");
            }
            Command::Quit { force: false, .. } => {
                log::warn!("Already processing a graceful stop.");
            }
            Command::Quit { no_exit, .. } => {
                log::warn!("Already processing a graceful stop, but force quit requested.");
                self.ctrl_tx.send(ControlEvent::Abort)?;
                self.exit = !no_exit;
            }
            Command::Pause { .. } => log::warn!("Cannot pause while stopping."),
            Command::Resume { .. } => log::warn!("Cannot resume while stopping."),
            Command::Exit if !self.schedulers_exited => {
                log::warn!("Please wait until the current jobs are stopped before exiting.")
            }
            Command::Exit => self.exit = true,
            Command::Clear => {}
            Command::Help => {}
        }

        Ok(NextState::Next(self))
    }
}
