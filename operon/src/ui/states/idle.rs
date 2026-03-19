use crate::scheduler::{ControlEvent, ControlEventSender};
use crate::ui::states::running::RunningState;
use crate::ui::states::{NextState, UiState};
use crate::ui::{Command, ProgressMap, UiError};

/// The initial state of the ui loop.
pub struct IdleState {
    /// Whether the `check` command has been run.
    checked: bool,
    ctrl_tx: ControlEventSender,
}

impl IdleState {
    pub fn new(ctrl_tx: ControlEventSender) -> Self {
        Self {
            checked: false,
            ctrl_tx,
        }
    }

    pub fn boxed(ctrl_tx: ControlEventSender) -> Box<Self> {
        Box::new(Self::new(ctrl_tx))
    }
}

impl UiState for IdleState {
    fn handle_progress(self: Box<Self>, _: ProgressMap) -> Result<NextState, UiError> {
        Ok(NextState::Next(self))
    }

    fn handle_command(mut self: Box<Self>, command: Command) -> Result<NextState, UiError> {
        match command {
            Command::Run { fresh, rebuild } => {
                self.ctrl_tx.send(ControlEvent::Run { fresh, rebuild })?;
                return Ok(NextState::Next(RunningState::boxed(self.ctrl_tx)));
            }
            Command::Check { .. } if self.checked => log::warn!("Already run a check."),
            Command::Check { mode } => {
                log::info!("Starting a consistency check of the remaining data.");
                self.ctrl_tx.send(ControlEvent::Check { mode })?;
                self.checked = true
            }
            Command::Quit { no_exit: true, .. } => {
                log::warn!("Cannot quit before the run has started.")
            }
            Command::Quit { .. } | Command::Exit => {
                self.ctrl_tx.send(ControlEvent::Abort)?;
                return Ok(NextState::Exit);
            }
            Command::Pause { .. } => log::warn!("Cannot pause before the run has started."),
            Command::Resume { .. } => log::warn!("Cannot resume before the run has started."),
            Command::Clear => {}
            Command::Help => {}
        }

        Ok(NextState::Next(self))
    }
}
