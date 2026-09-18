use std::error::Error;

use thiserror::Error as ThisError;

use crate::meta_storage::MetaStorageError;

/// A metadata result over the Postgres backend's error.
pub(super) type PsqlResult<T> = Result<T, MetaStorageError<PsqlMetaError>>;

/// Postgres-specific metadata errors.
///
/// # Stability
///
/// This enum is `#[non_exhaustive]`.
#[derive(Debug, ThisError)]
#[non_exhaustive]
pub enum PsqlMetaError {
    /// Internal database error.
    #[error("Database error: {}{}", .0, .0.source().map_or_else(String::new, |e| format!(", cause: {e}")))]
    Database(#[from] tokio_postgres::Error),
    /// Database pool error.
    #[error("Database pool error: {0}")]
    DatabasePool(#[from] deadpool_postgres::PoolError),
    /// Specified pool size is too small.
    #[error("Metadata pool size {0} is not allowed")]
    PoolSizeTooSmall(usize),
    /// Database pool construction error.
    #[error("Failed to build connection pool: {0}")]
    PoolBuild(#[from] deadpool_postgres::BuildError),
}

impl From<PsqlMetaError> for MetaStorageError<PsqlMetaError> {
    fn from(err: PsqlMetaError) -> Self {
        MetaStorageError::Backend(err)
    }
}
