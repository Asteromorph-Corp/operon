use crate::ui::states::{NextState, UiState};
use crate::ui::{Command, ProgressMap, UiError};

/// The state of the ui loop while handling `quit` command.
pub struct QuittingState {
    force: bool,
    exit: bool,
}

impl QuittingState {
    pub fn new(force: bool, exit: bool) -> Self {
        Self { force, exit }
    }

    pub fn boxed(force: bool, exit: bool) -> Box<Self> {
        Box::new(Self::new(force, exit))
    }
}

impl UiState for QuittingState {
    fn handle_progress(self: Box<Self>, _: ProgressMap) -> Result<NextState, UiError> {
        Ok(NextState::Next(self))
    }

    fn handle_command(self: Box<Self>, _: Command) -> Result<NextState, UiError> {
        Ok(NextState::Next(self))
    }
}
