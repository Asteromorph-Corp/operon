use crate::ui::{Command, ProgressMap, UiError};

mod idle;
mod quitting;
mod running;

pub use idle::IdleState;

pub enum NextState {
    Next(Box<dyn UiState>),
    Exit,
}

pub trait UiState {
    fn handle_command(self: Box<Self>, command: Command) -> Result<NextState, UiError>;
    fn handle_progress(self: Box<Self>, progress: ProgressMap) -> Result<NextState, UiError>;
}
