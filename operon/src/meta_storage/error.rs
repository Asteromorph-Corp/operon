use std::error::Error;
use std::num::TryFromIntError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MetaStorageError {
    #[error("Database error: {}{}", .0, .0.source().map_or_else(String::new, |e| format!(", cause: {e}")))]
    DatabaseError(#[from] tokio_postgres::Error),
    #[error("Database pool error: {0}")]
    DatabasePoolError(#[from] deadpool_postgres::PoolError),
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

impl From<deadpool_postgres::BuildError> for MetaStorageError {
    fn from(_: deadpool_postgres::BuildError) -> Self {
        Self::Internal("Failed to build connection pool")
    }
}
