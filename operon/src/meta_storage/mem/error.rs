use std::sync::PoisonError;

use thiserror::Error as ThisError;

use crate::meta_storage::{MetaResult, MetaStorageError};

/// A metadata result over the in-memory backend's error.
pub(super) type MemResult<T> = MetaResult<T, MemMetaError>;

/// In-memory-specific metadata errors.
///
/// # Stability
///
/// This enum is `#[non_exhaustive]`.
#[derive(Debug, ThisError)]
#[non_exhaustive]
pub enum MemMetaError {
    /// The read-write lock of this storage was poisoned due to an earlier panic.
    #[error("In-memory metadata store lock poisoned")]
    Poisoned,
}

impl From<MemMetaError> for MetaStorageError<MemMetaError> {
    fn from(err: MemMetaError) -> Self {
        MetaStorageError::Backend(err)
    }
}

impl<T> From<PoisonError<T>> for MetaStorageError<MemMetaError> {
    fn from(_: PoisonError<T>) -> Self {
        MemMetaError::Poisoned.into()
    }
}
