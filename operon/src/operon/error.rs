use thiserror::Error;

use crate::ui::UiError;

/// The error [`Operon::run`](crate::Operon::run) returns.
///
/// Operon runs its scheduler behind a UI that consumes backend errors,
/// so the error surface is limited to UI errors and critical failures in the scheduler.
#[derive(Debug, Error)]
pub enum OperonError {
    /// The scheduler failed to start, e.g. the metadata backend could not be constructed.
    // TODO(#79): drop this once the UI surfaces construction failures gracefully.
    // `Operon::run` would then return only `UI` and `SchedulerJoinError`.
    #[error("Startup error: {0}")]
    Startup(Box<dyn std::error::Error + Send + Sync>),
    /// Error in the terminal UI
    #[error("Terminal UI error: {0}")]
    UI(#[from] UiError),
    /// Error in the scheduler loop join
    #[error("Scheduler loop join error: {0}")]
    SchedulerJoinError(#[from] tokio::task::JoinError),
}

/// The default error type for user functions.
pub type UserError = Box<dyn std::error::Error + Send + Sync>;
