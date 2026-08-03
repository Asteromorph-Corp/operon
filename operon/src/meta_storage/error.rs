use std::num::TryFromIntError;

use thiserror::Error as ThisError;

/// A metadata result over a backend error `MErr`.
pub(crate) type MetaResult<T, MErr> = Result<T, MetaStorageError<MErr>>;

/// A metadata storage error.
///
/// Collects the backend-neutral domain errors any backend can raise, alongside the backend's own
/// error type `MErr` carried by [`Backend`](Self::Backend).
#[derive(Debug, ThisError)]
pub enum MetaStorageError<MErr> {
    #[error("Integer conversion error: {0}")]
    IntegerConversionError(#[from] TryFromIntError),
    #[error("Invalid run state: {0}")]
    InvalidRunState(String),
    // FIXME: the `job` fields of the two variants below hold a task id and need to be renamed at
    // v0.6.0, which breaks callers that match on them by name.
    #[error("Invalid explosion: Called `explode({dim})` on `{job}`, but `{dim}` was resolved.")]
    InvalidExplosion {
        job: &'static str,
        dim: &'static str,
    },
    #[error("Missing ticket summary for task '{job}'")]
    MissingTicketSummary { job: &'static str },
    #[error("Missing resolution for dimension `{dim}[{}]`", fmt_deps(.deps))]
    MissingResolution {
        dim: &'static str,
        deps: Vec<(&'static str, usize)>,
    },
    #[error("Internal error: {0}")]
    Internal(&'static str),
    #[error("Another Operon instance is already running against metadata schema `{0}`")]
    SchemaLocked(String),
    #[error(
        "Lost the advisory lock on metadata schema `{0}` (connection dropped or lock \
         otherwise released); stopping to avoid running unguarded"
    )]
    LockLost(String),
    #[error(transparent)]
    Backend(MErr),
}

fn fmt_deps(deps: &[(&'static str, usize)]) -> String {
    deps.iter()
        .map(|(name, val)| format!("{name} = {val}"))
        .collect::<Vec<_>>()
        .join(", ")
}

impl<MErr> MetaStorageError<MErr> {
    pub fn invalid_explosion(task: &'static str, dim: &'static str) -> Self {
        Self::InvalidExplosion { job: task, dim }
    }

    pub fn missing_ticket_summary(task: &'static str) -> Self {
        Self::MissingTicketSummary { job: task }
    }

    /// Remaps the backend error, passing the domain variants through unchanged.
    pub(crate) fn map_backend<U>(self, f: impl FnOnce(MErr) -> U) -> MetaStorageError<U> {
        match self {
            Self::IntegerConversionError(e) => MetaStorageError::IntegerConversionError(e),
            Self::InvalidRunState(s) => MetaStorageError::InvalidRunState(s),
            Self::InvalidExplosion { job, dim } => MetaStorageError::InvalidExplosion { job, dim },
            Self::MissingTicketSummary { job } => MetaStorageError::MissingTicketSummary { job },
            Self::MissingResolution { dim, deps } => {
                MetaStorageError::MissingResolution { dim, deps }
            }
            Self::Internal(s) => MetaStorageError::Internal(s),
            Self::SchemaLocked(s) => MetaStorageError::SchemaLocked(s),
            Self::LockLost(s) => MetaStorageError::LockLost(s),
            Self::Backend(e) => MetaStorageError::Backend(f(e)),
        }
    }
}
