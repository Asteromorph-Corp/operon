use async_trait::async_trait;

use crate::schema::RunFootprint;
use crate::storage::StorageResult;

/// The pipeline-independent half of an entity storage backend.
///
/// A backend implements this trait together with the `{PipelineName}Storage` trait that
/// [`define_operon!`](crate::define_operon) generates.
/// This trait specifically owns the backend's lifecycle and its run footprint,
/// while the generated half owns the per-entity accessors.
/// Every method reports failure as [`Self::Error`], which reaches the caller as
/// [`StorageError::Backend`](crate::error::StorageError::Backend).
///
/// # Run footprints
///
/// A [`RunFootprint`] records which run last wrote to this backend and the
/// [`RunState`](crate::RunState) it ended in.
/// Operon compares the footprint stored here against the one held by the metadata storage to decide
/// whether a previous run can be resumed, and treats a disagreement between the two as an aborted
/// run.
///
/// The three footprint methods default to no-ops, and a backend that does not implement them will
/// have recovery disabled.
/// Implement all three to resume a gracefully stopped run instead of recomputing it.
#[async_trait]
pub trait OperonStorage: Send + Sync + 'static {
    /// This storage backend's own error type, surfaced through
    /// [`StorageError::Backend`](crate::storage::StorageError::Backend).
    type Error: std::error::Error + Send + Sync + 'static;

    /// Prepares the backend to hold entity data, creating whatever tables, files or indices it
    /// needs.
    ///
    /// Operon calls this once as the scheduler starts, so it has to tolerate a backend that is
    /// already initialized.
    async fn init(&self) -> StorageResult<(), Self::Error>;

    /// Empties the backend, discarding every stored entity along with the run footprint while
    /// leaving the structures [`init`](Self::init) created in place.
    async fn clear(&self) -> StorageResult<(), Self::Error>;

    /// Reads the footprint of the run that last wrote to this backend, or `None` if there is none.
    async fn get_footprint(&self) -> StorageResult<Option<RunFootprint>, Self::Error> {
        Ok(None)
    }

    /// Records `footprint` as this backend's view of the current run, replacing any earlier one.
    async fn put_footprint(&self, _footprint: &RunFootprint) -> StorageResult<(), Self::Error> {
        Ok(())
    }

    /// Drops the recorded footprint, so that the next run starts from scratch.
    async fn clear_footprint(&self) -> StorageResult<(), Self::Error> {
        Ok(())
    }
}
