use thiserror::Error;

use crate::scheduler::ControlEventError;

#[derive(Debug, Error)]
pub enum UiError {
    #[error("Control event send error: {0}")]
    ControlEventSendFailed(#[from] ControlEventError),
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Log buffer error: {0}")]
    LogReceiveFailed(tokio::sync::broadcast::error::TryRecvError),
    #[error("Set logger error: {0}")]
    SetLoggerError(#[from] log::SetLoggerError),
    #[error("Other error: {0}")]
    Other(String),
}

impl From<tokio::sync::broadcast::error::TryRecvError> for UiError {
    fn from(e: tokio::sync::broadcast::error::TryRecvError) -> Self {
        UiError::LogReceiveFailed(e)
    }
}
