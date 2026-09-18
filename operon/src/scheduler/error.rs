use thiserror::Error;
use tokio::sync::AcquireError;
use tokio::task::JoinError;

use crate::meta_storage::{MemMetaError, MetaStorageError};
use crate::storage::StorageError;

/// The scheduler's error, generic over the service's error type `UErr`
/// ([`OperonService::Error`](crate::service::OperonService::Error)), the entity storage's error
/// type `SErr` ([`OperonStorage::Error`](crate::storage::OperonStorage::Error)), and the metadata
/// backend's error type `MErr` ([`MetaBackend::Error`](crate::meta_storage::MetaBackend::Error)).
///
/// # Stability
///
/// This enum is `#[non_exhaustive]`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SchedulerError<UErr, SErr, MErr> {
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
    #[error("Missing progress entry for task: {0}")]
    MissingProgressEntry(String),
    #[error("Other error: {0}")]
    Other(String),
}

impl<UErr, SErr, MErr> SchedulerError<UErr, SErr, MErr> {
    pub(crate) fn missing_progress(id: impl Into<String>) -> Self {
        Self::MissingProgressEntry(id.into())
    }

    pub(crate) fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

impl<UErr, SErr> SchedulerError<UErr, SErr, MemMetaError> {
    /// Re-types an error from the scratch in-memory store used during rebuild.
    pub(crate) fn during_rebuild<MErr>(self) -> SchedulerError<UErr, SErr, MErr> {
        match self {
            SchedulerError::UserError(e) => SchedulerError::UserError(e),
            SchedulerError::Storage(e) => SchedulerError::Storage(e),
            SchedulerError::MetaStorage(e) => SchedulerError::MetaStorage(e.during_rebuild()),
            SchedulerError::JoinFailed(e) => SchedulerError::JoinFailed(e),
            SchedulerError::SemaphoreAcquireFailed => SchedulerError::SemaphoreAcquireFailed,
            SchedulerError::ControlEventReceiveFailed => SchedulerError::ControlEventReceiveFailed,
            SchedulerError::PeerEventSendFailed => SchedulerError::PeerEventSendFailed,
            SchedulerError::InvalidPeerEventReceived(event, task) => {
                SchedulerError::InvalidPeerEventReceived(event, task)
            }
            SchedulerError::MissingProgressEntry(task) => {
                SchedulerError::MissingProgressEntry(task)
            }
            SchedulerError::Other(msg) => SchedulerError::Other(msg),
        }
    }
}

impl<UErr, SErr, MErr> From<AcquireError> for SchedulerError<UErr, SErr, MErr> {
    fn from(_: AcquireError) -> Self {
        SchedulerError::SemaphoreAcquireFailed
    }
}

impl<UErr, SErr, MErr> From<tokio::sync::watch::error::RecvError>
    for SchedulerError<UErr, SErr, MErr>
{
    fn from(_: tokio::sync::watch::error::RecvError) -> Self {
        SchedulerError::ControlEventReceiveFailed
    }
}
