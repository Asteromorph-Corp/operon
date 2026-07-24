use thiserror::Error;

#[derive(Debug, Error)]
pub enum UiError {
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Log buffer error: {0}")]
    LogReceiveFailed(tokio::sync::broadcast::error::TryRecvError),
    #[error("Set subscriber error: {0}")]
    SetSubscriberError(#[from] tracing::subscriber::SetGlobalDefaultError),
    #[error("Other error: {0}")]
    Other(String),
}

impl From<tokio::sync::broadcast::error::TryRecvError> for UiError {
    fn from(e: tokio::sync::broadcast::error::TryRecvError) -> Self {
        UiError::LogReceiveFailed(e)
    }
}
