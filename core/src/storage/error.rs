use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Other error: {0}")]
    Other(String),
}
