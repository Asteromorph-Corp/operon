use std::error::Error;
use std::fmt;
use std::num::TryFromIntError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Database error: {}{}", .0, .0.source().map_or_else(String::new, |e| format!(", cause: {e}")))]
    DatabaseError(#[from] tokio_postgres::Error),
    #[error("Database pool error: {0}")]
    DatabasePoolError(#[from] deadpool_postgres::PoolError),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("CSV error: {0}")]
    CsvError(#[from] csv::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Integer conversion error: {0}")]
    IntegerConversionError(#[from] TryFromIntError),
    #[error("Invalid run state: {0}")]
    InvalidRunState(String),
    #[error("Entity `{entity}[{}]` not found", fmt_dims(.dims))]
    EntityNotFound {
        entity: &'static str,
        dims: Vec<(&'static str, usize)>,
    },
    #[error("Entity `{entity}[{}]` expects {expected} elements, but only {actual} were found", fmt_dim_states(.dims))]
    EntityLengthMismatch {
        entity: &'static str,
        dims: Vec<(&'static str, DimState)>,
        expected: usize,
        actual: usize,
    },
    #[error("Internal error: {0}")]
    Internal(&'static str),
}

/// Describes the state of a dimension in an entity lookup.
#[derive(Debug)]
pub enum DimState {
    /// The dimension has a known value.
    Value(usize),
    /// The dimension is being aggregated over (collected).
    Aggregated,
    /// The dimension's value is not resolved at this point.
    Unresolved,
}

impl fmt::Display for DimState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Value(v) => write!(f, "{v}"),
            Self::Aggregated => write!(f, "*"),
            Self::Unresolved => write!(f, "_"),
        }
    }
}

fn fmt_dims(dims: &[(&'static str, usize)]) -> String {
    dims.iter()
        .map(|(name, val)| format!("{name} = {val}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn fmt_dim_states(dims: &[(&'static str, DimState)]) -> String {
    dims.iter()
        .map(|(name, state)| format!("{name} = {state}"))
        .collect::<Vec<_>>()
        .join(", ")
}

impl From<csv::IntoInnerError<csv::Writer<Vec<u8>>>> for StorageError {
    fn from(err: csv::IntoInnerError<csv::Writer<Vec<u8>>>) -> Self {
        StorageError::IoError(err.into_error())
    }
}

impl From<deadpool_postgres::BuildError> for StorageError {
    fn from(_: deadpool_postgres::BuildError) -> Self {
        Self::Internal("Failed to build connection pool")
    }
}
