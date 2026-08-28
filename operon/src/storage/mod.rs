mod storage;
pub use storage::OperonStorage;

pub mod mem;
pub mod psql;

mod options;
#[allow(deprecated)]
pub use options::StorageOptions;

mod error;
pub use error::{DimState, StorageError, StorageResult};
