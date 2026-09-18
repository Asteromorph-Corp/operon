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
    /// Error in the user-provided function.
    #[error("Error in user provided function: {0}")]
    UserError(UErr),
    /// Error in the entity storage backend.
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError<SErr>),
    /// Error in the metadata storage backend.
    #[error("Metadata storage error: {0}")]
    MetaStorage(#[from] MetaStorageError<MErr>),
    /// Failed to join futures.
    #[error("Join failed: {0}")]
    JoinFailed(#[from] JoinError),
    /// Failed to acquire per-task semaphore.
    #[error("Failed to acquire semaphore")]
    SemaphoreAcquireFailed,
    /// Failed to receive control event.
    #[error("Failed to receive control event")]
    ControlEventReceiveFailed,
    /// Failed to send peer event.
    #[error("Failed to send peer event")]
    PeerEventSendFailed,
    /// Received a peer event irrelevant to the current task.
    #[error("Irrelevant {0} event received in `{1}` scheduler")]
    InvalidPeerEventReceived(&'static str, &'static str),
    /// Missing progress entry for a task.
    #[error("Missing progress entry for task: {0}")]
    MissingProgressEntry(String),
    /// Unspecified error.
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
