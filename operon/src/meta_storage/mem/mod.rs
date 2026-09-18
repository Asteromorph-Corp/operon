//! The in-memory implementation of the metadata backend.
//!
//! Holds a run's metadata in process. It is volatile and non-transactional, trading durability for
//! speed on work that is rebuilt from scratch on failure.

mod client;
pub(crate) use client::{MemClient, MemConn, MemTx};

mod error;
pub use error::MemMetaError;

mod options;
pub use options::MemMetaStorageOptions;

mod store;

mod storage;
pub use storage::MemMetaStorage;

mod resolution;
mod ticket;

pub(crate) use resolution::MemResolutionQueryBuilder;
pub(crate) use ticket::MemTicketQueryBuilder;
