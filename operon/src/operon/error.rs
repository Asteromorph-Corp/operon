use thiserror::Error;

use crate::ui::UiError;

/// The error [`Operon::run`](crate::Operon::run) returns.
///
/// Operon runs its scheduler behind a UI that consumes backend errors,
/// so the error surface is limited to UI errors and scheduler panics.
#[derive(Debug, Error)]
pub enum OperonError {
    /// Error in the terminal UI
    #[error("Terminal UI error: {0}")]
    UI(#[from] UiError),
    /// Panic in the scheduler loop
    #[error("Scheduler loop panic: {0}")]
    SchedulerPanic(#[from] tokio::task::JoinError),
}

/// The default error type for user functions.
pub type UserError = Box<dyn std::error::Error + Send + Sync>;
