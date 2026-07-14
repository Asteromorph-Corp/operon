use std::error::Error;

use thiserror::Error as ThisError;

use crate::meta_storage::MetaStorageError;

/// A metadata result over the Postgres backend's error.
pub(super) type PsqlResult<T> = Result<T, MetaStorageError<PsqlMetaError>>;

/// Postgres-specific metadata errors.
#[derive(Debug, ThisError)]
pub enum PsqlMetaError {
    #[error("Database error: {}{}", .0, .0.source().map_or_else(String::new, |e| format!(", cause: {e}")))]
    Database(#[from] tokio_postgres::Error),
    #[error("Database pool error: {0}")]
    DatabasePool(#[from] deadpool_postgres::PoolError),
    #[error("Metadata pool size {0} is not allowed")]
    PoolSizeTooSmall(usize),
    #[error("Failed to build connection pool: {0}")]
    PoolBuild(#[from] deadpool_postgres::BuildError),
}

impl From<PsqlMetaError> for MetaStorageError<PsqlMetaError> {
    fn from(err: PsqlMetaError) -> Self {
        MetaStorageError::Backend(err)
    }
}
