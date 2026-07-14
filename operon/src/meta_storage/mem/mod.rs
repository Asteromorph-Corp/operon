//! The in-memory implementation of the metadata backend.
//!
//! Holds a run's metadata in process. It is volatile and non-transactional, trading durability for
//! speed on work that is rebuilt from scratch on failure.

mod client;
pub use client::{MemClient, MemConn, MemTx};

mod error;

mod options;
pub use options::MemMetaStorageOptions;

mod store;

mod storage;
pub use storage::MemMetaStorage;

mod resolution;
mod ticket;

pub use resolution::MemResolutionQueryBuilder;
pub use ticket::MemTicketQueryBuilder;
