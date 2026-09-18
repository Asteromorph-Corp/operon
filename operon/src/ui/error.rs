use thiserror::Error;

/// An error from Operon's UI.
///
/// # Stability
///
/// This enum is `#[non_exhaustive]`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum UiError {
    /// A file or stream I/O error.
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    /// Could not set global `tracing` subscriber.
    #[error("Set subscriber error: {0}")]
    SetSubscriberError(#[from] tracing::subscriber::SetGlobalDefaultError),
    /// Unspecified error.
    #[error("Other error: {0}")]
    Other(String),
}
