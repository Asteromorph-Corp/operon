use thiserror::Error;

use crate::{meta_storage::MetaStorageError, scheduler::SchedulerError, ui::UiError};

// #[derive(Debug, Error)]
// pub enum StorageError {
//     #[error("Other error: {0}")]
//     Other(String),
// }

// #[derive(Debug, Error)]
// pub enum UserError {
//     #[error("Other error: {0}")]
//     Other(String),
// }

/// Error type returned by Operon.
#[derive(Debug, Error)]
pub enum OperonError {
    /// Error in a storage operation
    #[error("Storage error: {0}")]
    Storage(::anyhow::Error),
    /// Error in the scheduler
    #[error("Scheduler error: {0}")]
    Scheduler(SchedulerError),
    /// Error in a user function
    #[error("User function error: {0}")]
    User(::anyhow::Error),
    /// Error in the metadata storage
    #[error("Metadata storage error: {0}")]
    MetaStorage(#[from] MetaStorageError),
    /// Error in the terminal UI
    #[error("Terminal UI error: {0}")]
    UI(#[from] UiError),
    /// Error caused by missing data
    #[error("Data expected but not found: {0}")]
    NotFound(String),
    /// Tried to resolve a ticket with an irrelevant resolution
    #[error("Invalid resolution: {0}")]
    InvalidResolution(String),
}

impl OperonError {
    pub fn not_found(data: impl Into<String>) -> Self {
        OperonError::NotFound(data.into())
    }

    pub fn invalid_resolution(resolution: impl Into<String>) -> Self {
        OperonError::InvalidResolution(resolution.into())
    }
}

impl From<SchedulerError> for OperonError {
    fn from(e: SchedulerError) -> Self {
        match e {
            SchedulerError::MetaStorage(e) => OperonError::MetaStorage(e),
            other => {
                // Convert other scheduler errors to OperonError::Scheduler
                OperonError::Scheduler(other)
            }
        }
    }
}
