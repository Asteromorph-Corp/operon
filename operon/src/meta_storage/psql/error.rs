use std::error::Error;

use thiserror::Error as ThisError;

use crate::meta_storage::MetaStorageError;

/// Postgres-specific metadata errors, kept out of the backend-neutral [`MetaStorageError`] so its
/// `tokio_postgres` / `deadpool_postgres` dependencies stay behind this module.
#[derive(Debug, ThisError)]
pub enum PsqlMetaError {
    #[error("Database error: {}{}", .0, .0.source().map_or_else(String::new, |e| format!(", cause: {e}")))]
    Database(#[from] tokio_postgres::Error),
    #[error("Database pool error: {0}")]
    DatabasePool(#[from] deadpool_postgres::PoolError),
    #[error("Metadata pool size {0} is not allowed")]
    PoolSizeTooSmall(usize),
    #[error("Failed to build connection pool")]
    PoolBuild,
}

impl From<deadpool_postgres::BuildError> for PsqlMetaError {
    fn from(_: deadpool_postgres::BuildError) -> Self {
        Self::PoolBuild
    }
}

impl From<PsqlMetaError> for MetaStorageError {
    fn from(err: PsqlMetaError) -> Self {
        MetaStorageError::Backend(Box::new(err))
    }
}
