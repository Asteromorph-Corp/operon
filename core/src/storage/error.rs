use std::num::TryFromIntError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] deadpool_postgres::tokio_postgres::Error),
    #[error("Database pool error: {0}")]
    DatabasePoolError(#[from] deadpool_postgres::PoolError),
    #[error("Failed to build connection pool: {0}")]
    PoolBuildError(#[from] deadpool_postgres::BuildError),
    #[error("Database uri parse error: {0}")]
    DatabaseUriParseError(String),
    #[error("Integer conversion error: {0}")]
    IntegerConversionError(#[from] TryFromIntError),
    #[error("Error serializing/deserializing json: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("CSV error: {0}")]
    CsvError(#[from] csv::Error),
    #[error("Writer into inner error: {0}")]
    WriterError(Box<csv::IntoInnerError<csv::Writer<Vec<u8>>>>),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Other error: {0}")]
    Other(String),
}

impl From<csv::IntoInnerError<csv::Writer<Vec<u8>>>> for StorageError {
    fn from(err: csv::IntoInnerError<csv::Writer<Vec<u8>>>) -> Self {
        StorageError::WriterError(Box::new(err))
    }
}
