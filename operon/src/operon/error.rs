use thiserror::Error;

use crate::meta_storage::MetaStorageError;
use crate::scheduler::SchedulerError;
use crate::storage::StorageError;
use crate::ui::UiError;

/// Error type returned by Operon, generic over the service's error type `UErr`
/// ([`OperonService::Error`](crate::service::OperonService::Error)), the entity storage's error
/// type `SErr` ([`OperonStorage::Error`](crate::storage::OperonStorage::Error)), and the metadata
/// backend's error type `MErr` ([`MetaBackend::Error`](crate::meta_storage::MetaBackend::Error)).
#[derive(Debug, Error)]
pub enum OperonError<UErr, SErr, MErr> {
    /// Error in the scheduler
    #[error("Scheduler error: {0}")]
    Scheduler(SchedulerError<UErr, SErr, MErr>),
    /// Error in a user function
    #[error("User function error: {0}")]
    User(UErr),
    /// Error in a storage operation
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError<SErr>),
    /// Error in the metadata storage
    #[error("Metadata storage error: {0}")]
    MetaStorage(#[from] MetaStorageError<MErr>),
    /// Error in the terminal UI
    #[error("Terminal UI error: {0}")]
    UI(#[from] UiError),
    /// Error in the scheduler loop join
    #[error("Scheduler loop join error: {0}")]
    SchedulerJoinError(#[from] tokio::task::JoinError),
}

/// The default error type for user functions.
pub type UserError = Box<dyn std::error::Error + Send + Sync>;

impl<UErr, SErr, MErr> From<SchedulerError<UErr, SErr, MErr>> for OperonError<UErr, SErr, MErr> {
    fn from(e: SchedulerError<UErr, SErr, MErr>) -> Self {
        match e {
            SchedulerError::Storage(e) => OperonError::Storage(e),
            SchedulerError::MetaStorage(e) => OperonError::MetaStorage(e),
            SchedulerError::UserError(e) => OperonError::User(e),
            // Convert other scheduler errors to OperonError::Scheduler
            other => OperonError::Scheduler(other),
        }
    }
}
