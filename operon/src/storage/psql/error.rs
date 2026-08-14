use std::error::Error;

use thiserror::Error as ThisError;

use crate::storage::StorageError;

/// A storage result over the Postgres backend's error.
pub(crate) type PsqlStorageResult<T> = Result<T, StorageError<PsqlStorageError>>;

/// Postgres-specific entity-storage errors.
#[derive(Debug, ThisError)]
pub enum PsqlStorageError {
    #[error("Database error: {}{}", .0, .0.source().map_or_else(String::new, |e| format!(", cause: {e}")))]
    Database(#[from] tokio_postgres::Error),
    #[error("Database pool error: {0}")]
    DatabasePool(#[from] deadpool_postgres::PoolError),
    #[error("Failed to build connection pool: {0}")]
    PoolBuild(#[from] deadpool_postgres::BuildError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<PsqlStorageError> for StorageError<PsqlStorageError> {
    fn from(err: PsqlStorageError) -> Self {
        StorageError::Backend(err)
    }
}

/// Converts a raw driver error into the wrapped storage error in one hop.
macro_rules! from_backend {
    ($($ty:ty),* $(,)?) => {$(
        impl From<$ty> for StorageError<PsqlStorageError> {
            fn from(err: $ty) -> Self {
                StorageError::Backend(PsqlStorageError::from(err))
            }
        }
    )*};
}

from_backend! {
    tokio_postgres::Error,
    deadpool_postgres::PoolError,
    deadpool_postgres::BuildError,
    serde_json::Error,
    csv::Error,
    std::io::Error,
}

impl From<csv::IntoInnerError<csv::Writer<Vec<u8>>>> for StorageError<PsqlStorageError> {
    fn from(err: csv::IntoInnerError<csv::Writer<Vec<u8>>>) -> Self {
        StorageError::Backend(PsqlStorageError::Io(err.into_error()))
    }
}
