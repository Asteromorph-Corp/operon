use std::num::TryFromIntError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MetaStorageError {
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
    #[error("Invalid explosion: {0}")]
    InvalidExplosion(String),
    #[error("Missing resolution: {0}")]
    MissingResolution(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Other error: {0}")]
    Other(String),
}
