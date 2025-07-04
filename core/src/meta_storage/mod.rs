#[allow(clippy::module_inception)]
mod meta_storage;
pub use meta_storage::*;

mod meta_client;
pub use meta_client::*;

mod init_schema;
pub use init_schema::*;

/// Operations for footprinting the PSQL metadata storage.
/// Given a connection with an optional schema, this module provides operations footprint the metadata storage.
mod footprint;
pub use footprint::*;

mod ticket_summary;
pub use ticket_summary::*;

mod options;
pub use options::*;

mod error;
pub use error::*;
