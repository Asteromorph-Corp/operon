use std::sync::PoisonError;

use thiserror::Error as ThisError;

use crate::meta_storage::{MetaResult, MetaStorageError};

/// A metadata result over the in-memory backend's error.
pub(super) type MemResult<T> = MetaResult<T, MemMetaError>;

/// In-memory-specific metadata errors.
#[derive(Debug, ThisError)]
pub enum MemMetaError {
    #[error("In-memory metadata store lock poisoned")]
    Poisoned,
}

impl From<MemMetaError> for MetaStorageError<MemMetaError> {
    fn from(err: MemMetaError) -> Self {
        MetaStorageError::Backend(err)
    }
}

/// Maps a poisoned store lock onto the backend's error.
pub(super) fn poisoned<T>(_: PoisonError<T>) -> MetaStorageError<MemMetaError> {
    MemMetaError::Poisoned.into()
}
