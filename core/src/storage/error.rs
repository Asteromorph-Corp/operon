use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Not found")]
    NotFound,
    #[error("Other error: {0}")]
    Other(String),
}
