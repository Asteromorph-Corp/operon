use std::fmt;
use std::num::TryFromIntError;

use thiserror::Error;

/// A storage result over a backend error `SErr`.
pub type StorageResult<T, SErr> = Result<T, StorageError<SErr>>;

/// An entity-storage error.
///
/// Collects the backend-neutral domain errors any storage backend can raise, alongside the
/// backend's own error type `SErr` carried by [`Backend`](Self::Backend).
#[derive(Debug, Error)]
pub enum StorageError<SErr> {
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
    #[error(transparent)]
    Backend(SErr),
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
