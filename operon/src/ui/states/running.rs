use crate::ui::states::{NextState, UiState};
use crate::ui::{Command, ProgressMap, UiError};

/// The running state of the ui loop.
pub struct RunningState;

impl RunningState {
    pub fn new() -> Self {
        Self
    }

    pub fn boxed() -> Box<Self> {
        Box::new(Self::new())
    }
}

impl UiState for RunningState {
    fn handle_progress(self: Box<Self>, _: ProgressMap) -> Result<NextState, UiError> {
        Ok(NextState::Next(self))
    }

    fn handle_command(self: Box<Self>, _: Command) -> Result<NextState, UiError> {
        Ok(NextState::Next(self))
    }
}
