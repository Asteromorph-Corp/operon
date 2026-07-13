//! The Postgres implementation of the metadata backend.
//!
//! Everything Postgres-specific about the metadata storage — connection pooling, the schema
//! advisory lock, SQL generation, and the wire (de)serialization of the domain types — lives
//! behind this module.
//! The rest of the crate speaks to it only through the backend-agnostic
//! [`MetaStorage`](crate::meta_storage::MetaStorage) /
//! [`MetaClient`](crate::meta_storage::MetaClient) enums.

mod client;
pub use client::{PsqlClient, PsqlConn, PsqlTx};

mod options;
pub use options::PsqlMetaStorageOptions;

mod storage;
pub use storage::PsqlMetaStorage;

mod footprint;
mod resolution;
mod schema;
mod ticket;

pub use resolution::PsqlResolutionQueryBuilder;
pub use ticket::PsqlTicketQueryBuilder;
