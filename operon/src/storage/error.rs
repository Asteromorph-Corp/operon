use std::fmt;
use std::num::TryFromIntError;

use thiserror::Error;

/// A storage result over a backend error `SErr`.
pub type StorageResult<T, SErr> = Result<T, StorageError<SErr>>;

/// An entity-storage error.
///
/// Collects the backend-neutral domain errors any storage backend can raise, alongside the
/// backend's own error type `SErr` carried by [`Backend`](Self::Backend).
///
/// # Stability
///
/// This enum is `#[non_exhaustive]`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StorageError<SErr> {
    /// An integer conversion error.
    #[error("Integer conversion error: {0}")]
    IntegerConversionError(#[from] TryFromIntError),
    /// Encountered an invalid run state.
    #[error("Invalid run state: {0}")]
    InvalidRunState(String),
    /// An entity entry was not found in the storage backend.
    #[error("Entity `{entity}[{}]` not found", fmt_dims(.dims))]
    EntityNotFound {
        /// The name of the entity that was not found.
        entity: &'static str,
        /// The coordinates to the entity entry that was expected.
        dims: Vec<(&'static str, usize)>,
    },
    /// The found entity entries had a count less than expected.
    #[error("Entity `{entity}[{}]` expects {expected} elements, but only {actual} were found", fmt_dim_states(.dims))]
    EntityLengthMismatch {
        /// The name of the entities that were not found.
        entity: &'static str,
        /// The coordinates to the entity entries that were expected.
        dims: Vec<(&'static str, DimState)>,
        /// The expected number of entity entries.
        expected: usize,
        /// The actual number of entity entries found.
        actual: usize,
    },
    /// An unspecified error from the storage backend.
    #[error("Internal error: {0}")]
    Internal(&'static str),
    /// An error from a specific storage backend.
    #[error(transparent)]
    Backend(SErr),
}

/// Describes the state of a dimension in an entity lookup.
///
/// # Stability
///
/// This enum is `#[non_exhaustive]`: adding a state variant must stay a non-breaking change.
#[derive(Debug)]
#[non_exhaustive]
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
