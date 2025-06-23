use thiserror::Error;

use crate::{
    meta_storage::MetaStorageError,
    scheduler::{ControlEvent, RecoveryState, RecoveryStateSendError},
    storage::StorageError,
};

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Metadata storage error: {0}")]
    MetaStorage(#[from] MetaStorageError),
    #[error("Failed to send recovery state: {0}")]
    RecoverySendFailed(RecoveryState),
    #[error("Unexpected control event: {0:?}")]
    UnexpectedControlEvent(ControlEvent),
    #[error("Failed to receive control event")]
    ControlEventReceiveFailed,
    #[error("Other error: {0}")]
    Other(String),
}

impl From<RecoveryStateSendError> for SchedulerError {
    fn from(err: RecoveryStateSendError) -> Self {
        SchedulerError::RecoverySendFailed(err.0)
    }
}

impl From<tokio::sync::watch::error::RecvError> for SchedulerError {
    fn from(_: tokio::sync::watch::error::RecvError) -> Self {
        SchedulerError::ControlEventReceiveFailed
    }
}
