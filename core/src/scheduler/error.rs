use thiserror::Error;

use crate::{
    meta_storage::MetaStorageError,
    scheduler::{ControlEvent, RecoveryState},
};

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("Metadata storage error: {0}")]
    MetaStorage(#[from] MetaStorageError),
    #[error("Failed to send recovery state: {0}")]
    RecoverySendFailed(RecoveryState),
    #[error("Unexpected control event: {0:?}")]
    UnexpectedControlEvent(ControlEvent),
    #[error("Other error: {0}")]
    Other(String),
}
