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
    #[error("Invalid explosion: Called `explode({dim})` on `{job}`, but `{dim}` was resolved.")]
    InvalidExplosion {
        job: &'static str,
        dim: &'static str,
    },
    #[error("Missing ticket summary for job '{job}'")]
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
    pub fn invalid_explosion(job: &'static str, dim: &'static str) -> Self {
        Self::InvalidExplosion { job, dim }
    }

    pub fn missing_ticket_summary(job: &'static str) -> Self {
        Self::MissingTicketSummary { job }
    }
}
