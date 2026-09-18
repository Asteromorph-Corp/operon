use std::num::TryFromIntError;

use thiserror::Error as ThisError;

use crate::meta_storage::MemMetaError;

/// A metadata result over a backend error `MErr`.
pub(crate) type MetaResult<T, MErr> = Result<T, MetaStorageError<MErr>>;

/// A metadata storage error.
///
/// Collects the backend-neutral domain errors any backend can raise, alongside the backend's own
/// error type `MErr` carried by [`Backend`](Self::Backend).
///
/// # Stability
///
/// This enum is `#[non_exhaustive]`.
#[derive(Debug, ThisError)]
#[non_exhaustive]
pub enum MetaStorageError<MErr> {
    /// An integer conversion error.
    #[error("Integer conversion error: {0}")]
    IntegerConversionError(#[from] TryFromIntError),
    /// Encountered an invalid run state.
    #[error("Invalid run state: {0}")]
    InvalidRunState(String),
    /// Invoked an explosion on an already-resolved dimension.
    #[error("Invalid explosion: Called `explode({dim})` on `{task}`, but `{dim}` was resolved.")]
    InvalidExplosion {
        /// The name of the task on which the invalid explosion was invoked.
        task: &'static str,
        /// The dimension name this explosion tried to resolve.
        dim: &'static str,
    },
    /// Missing ticket summary for a task.
    #[error("Missing ticket summary for task '{task}'")]
    MissingTicketSummary {
        /// The name of the task whose ticket summary is missing.
        task: &'static str,
    },
    /// Missing resolution for a dimension.
    #[error("Missing resolution for dimension `{dim}[{}]`", fmt_deps(.deps))]
    MissingResolution {
        /// The name of the dimension that is missing a resolution.
        dim: &'static str,
        /// The ancestor coordinates of this resolution.
        deps: Vec<(&'static str, usize)>,
    },
    /// Unspecified internal error.
    #[error("Internal error: {0}")]
    Internal(&'static str),
    /// Tried to run two instances of Operon against the same metadata schema.
    #[error("Another Operon instance is already running against metadata schema `{0}`")]
    SchemaLocked(String),
    /// Lost the advisory lock on the metadata schema.
    #[error(
        "Lost the advisory lock on metadata schema `{0}` (connection dropped or lock \
         otherwise released); stopping to avoid running unguarded"
    )]
    LockLost(String),
    /// An error from a specific metadata backend.
    #[error(transparent)]
    Backend(MErr),
    /// An error from the scratch in-memory store during the rebuild phase.
    #[error("Error from scratch in-memory store used during rebuild: {0}")]
    RebuildBackend(MemMetaError),
}

fn fmt_deps(deps: &[(&'static str, usize)]) -> String {
    deps.iter()
        .map(|(name, val)| format!("{name} = {val}"))
        .collect::<Vec<_>>()
        .join(", ")
}

impl<MErr> MetaStorageError<MErr> {
    pub(crate) fn invalid_explosion(task: &'static str, dim: &'static str) -> Self {
        Self::InvalidExplosion { task, dim }
    }

    pub(crate) fn missing_ticket_summary(task: &'static str) -> Self {
        Self::MissingTicketSummary { task }
    }

    /// Re-types an error from the scratch in-memory store used during rebuild.
    pub(crate) fn and_then_backend<U>(
        self,
        f: impl FnOnce(MErr) -> MetaStorageError<U>,
    ) -> MetaStorageError<U> {
        match self {
            Self::IntegerConversionError(e) => MetaStorageError::IntegerConversionError(e),
            Self::InvalidRunState(s) => MetaStorageError::InvalidRunState(s),
            Self::InvalidExplosion { task, dim } => {
                MetaStorageError::InvalidExplosion { task, dim }
            }
            Self::MissingTicketSummary { task } => MetaStorageError::MissingTicketSummary { task },
            Self::MissingResolution { dim, deps } => {
                MetaStorageError::MissingResolution { dim, deps }
            }
            Self::Internal(s) => MetaStorageError::Internal(s),
            Self::RebuildBackend(e) => MetaStorageError::RebuildBackend(e),
            Self::SchemaLocked(s) => MetaStorageError::SchemaLocked(s),
            Self::LockLost(s) => MetaStorageError::LockLost(s),
            Self::Backend(e) => f(e),
        }
    }

    /// Remaps the backend error, passing the domain variants through unchanged.
    pub(crate) fn map_backend<U>(self, f: impl FnOnce(MErr) -> U) -> MetaStorageError<U> {
        self.and_then_backend(|e| MetaStorageError::Backend(f(e)))
    }
}

impl MetaStorageError<MemMetaError> {
    /// Re-types an error from the scratch in-memory store used during rebuild.
    pub(crate) fn during_rebuild<U>(self) -> MetaStorageError<U> {
        self.and_then_backend(MetaStorageError::RebuildBackend)
    }
}
