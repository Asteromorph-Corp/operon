use thiserror::Error;
use tokio::sync::AcquireError;
use tokio::task::JoinError;

use crate::meta_storage::MetaStorageError;
use crate::storage::StorageError;

/// The scheduler's error, generic over the metadata backend's error type `MErr`
/// ([`MetaBackend::Error`](crate::meta_storage::MetaBackend::Error)), the entity storage's error
/// type `SErr` ([`OperonStorage::Error`](crate::storage::OperonStorage::Error)), and the service's
/// error type `UErr` ([`OperonService::Error`](crate::service::OperonService::Error)).
#[derive(Debug, Error)]
pub enum SchedulerError<MErr, SErr, UErr> {
    #[error("Error in user provided function: {0}")]
    UserError(UErr),
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError<SErr>),
    #[error("Metadata storage error: {0}")]
    MetaStorage(#[from] MetaStorageError<MErr>),
    #[error("Join failed: {0}")]
    JoinFailed(#[from] JoinError),
    #[error("Failed to acquire semaphore")]
    SemaphoreAcquireFailed,
    #[error("Failed to receive control event")]
    ControlEventReceiveFailed,
    #[error("Failed to send peer event")]
    PeerEventSendFailed,
    #[error("Irrelevant {0} event received in `{1}` scheduler")]
    InvalidPeerEventReceived(&'static str, &'static str),
    #[error("Missing progress entry for job: {0}")]
    MissingProgressEntry(String),
    #[error("Other error: {0}")]
    Other(String),
}

impl<MErr, SErr, UErr> SchedulerError<MErr, SErr, UErr> {
    pub(crate) fn missing_progress(id: impl Into<String>) -> Self {
        Self::MissingProgressEntry(id.into())
    }

    pub(crate) fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

impl<MErr, SErr, UErr> From<AcquireError> for SchedulerError<MErr, SErr, UErr> {
    fn from(_: AcquireError) -> Self {
        SchedulerError::SemaphoreAcquireFailed
    }
}

impl<MErr, SErr, UErr> From<tokio::sync::watch::error::RecvError>
    for SchedulerError<MErr, SErr, UErr>
{
    fn from(_: tokio::sync::watch::error::RecvError) -> Self {
        SchedulerError::ControlEventReceiveFailed
    }
}
