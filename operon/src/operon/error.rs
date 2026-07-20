use thiserror::Error;

use crate::meta_storage::MetaStorageError;
use crate::scheduler::SchedulerError;
use crate::storage::StorageError;
use crate::ui::UiError;

/// Error type returned by Operon, generic over the metadata backend's error type `MErr`
/// ([`MetaBackend::Error`](crate::meta_storage::MetaBackend::Error)) and the entity storage's error
/// type `SErr` ([`OperonStorage::Error`](crate::storage::OperonStorage::Error)).
#[derive(Debug, Error)]
pub enum OperonError<MErr, SErr> {
    /// Error in the scheduler
    #[error("Scheduler error: {0}")]
    Scheduler(SchedulerError<MErr, SErr>),
    /// Error in a user function
    #[error("User function error: {0}")]
    User(UserError),
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

pub type UserError = Box<dyn std::error::Error + Send + Sync>;

impl<MErr, SErr> From<SchedulerError<MErr, SErr>> for OperonError<MErr, SErr> {
    fn from(e: SchedulerError<MErr, SErr>) -> Self {
        match e {
            SchedulerError::Storage(e) => OperonError::Storage(e),
            SchedulerError::MetaStorage(e) => OperonError::MetaStorage(e),
            SchedulerError::UserError(e) => OperonError::User(e),
            // Convert other scheduler errors to OperonError::Scheduler
            other => OperonError::Scheduler(other),
        }
    }
}
