use std::error::Error;
use std::num::TryFromIntError;

use thiserror::Error as ThisError;

/// The backend-neutral metadata error.
///
/// Backend-specific failures arrive through [`Backend`](Self::Backend); each backend converts its
/// own error into it via `Into<MetaStorageError>`.
#[derive(Debug, ThisError)]
pub enum MetaStorageError {
    #[error("Backend error: {0}")]
    Backend(Box<dyn Error + Send + Sync + 'static>),
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
}

fn fmt_deps(deps: &[(&'static str, usize)]) -> String {
    deps.iter()
        .map(|(name, val)| format!("{name} = {val}"))
        .collect::<Vec<_>>()
        .join(", ")
}

impl MetaStorageError {
    pub fn invalid_explosion(job: &'static str, dim: &'static str) -> Self {
        Self::InvalidExplosion { job, dim }
    }

    pub fn missing_ticket_summary(job: &'static str) -> Self {
        Self::MissingTicketSummary { job }
    }
}
