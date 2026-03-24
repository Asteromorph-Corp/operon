use thiserror::Error;
use tokio::sync::AcquireError;
use tokio::task::JoinError;

use crate::meta_storage::MetaStorageError;
use crate::scheduler::{ControlEvent, RecoveryState, RecoveryStateSendError};
use crate::storage::StorageError;
use crate::ui::UiError;

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Metadata storage error: {0}")]
    MetaStorage(#[from] MetaStorageError),
    #[error("UI Error: {0}")]
    Ui(#[from] UiError),
    #[error("Join failed: {0}")]
    JoinFailed(#[from] JoinError),
    #[error("Failed to send recovery state: {0}")]
    RecoverySendFailed(RecoveryState),
    #[error("Unexpected control event: {0:?}")]
    UnexpectedControlEvent(ControlEvent),
    #[error("Failed to acquire semaphore")]
    SemaphoreAcquireFailed,
    #[error("Failed to receive control event")]
    ControlEventReceiveFailed,
    #[error("Failed to send peer event")]
    PeerEventSendFailed,
    #[error("Failed to send scheduler state: {0}")]
    SchedulerStateSendFailed(bool),
    #[error("Irrelevant {0} event received in `{1}` scheduler")]
    InvalidPeerEventReceived(&'static str, &'static str),
    #[error("Tried to send a PeerEvent through a downgraded sender")]
    SendThroughDowngradedSender,
    #[error("Error in user provided function: {0}")]
    UserError(Box<dyn std::error::Error + Send + Sync>),
    #[error("Missing progress entry for job: {0}")]
    MissingProgressEntry(String),
    #[error("Other error: {0}")]
    Other(String),
}

impl SchedulerError {
    pub(crate) fn missing_progress(id: impl Into<String>) -> Self {
        Self::MissingProgressEntry(id.into())
    }

    pub(crate) fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

impl From<RecoveryStateSendError> for SchedulerError {
    fn from(err: RecoveryStateSendError) -> Self {
        SchedulerError::RecoverySendFailed(err.0)
    }
}

impl From<AcquireError> for SchedulerError {
    fn from(_: AcquireError) -> Self {
        SchedulerError::SemaphoreAcquireFailed
    }
}

impl From<tokio::sync::watch::error::RecvError> for SchedulerError {
    fn from(_: tokio::sync::watch::error::RecvError) -> Self {
        SchedulerError::ControlEventReceiveFailed
    }
}
