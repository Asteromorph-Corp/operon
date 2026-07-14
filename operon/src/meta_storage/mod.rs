mod any;
pub use any::{AnyBackend, AnyBackendError};

#[cfg(test)]
mod differential;

mod backend;
pub use backend::{
    MetaBackend, MetaClientApi, MetaConnApi, MetaResolutionApi, MetaTicketApi, MetaTxApi,
};

mod options;
pub use options::MetaBackendOptions;

mod error;
pub(crate) use error::MetaResult;
pub use error::MetaStorageError;

mod mem;
pub use mem::MemMetaStorageOptions;

mod psql;
pub use psql::{PsqlMetaError, PsqlMetaStorageOptions};
