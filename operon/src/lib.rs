#![allow(clippy::module_inception)]

mod logger;
mod meta_storage;
mod operon;
mod scheduler;
mod schema;
mod service;
mod storage;
mod ui;
mod utils;

pub use meta_storage::AnyBackend;
pub use operon::Operon;
pub use operon_macros::define_operon;
pub use schema::{Direction, Entity};
pub use service::OperonService;
pub use storage::OperonStorage;

pub mod error {
    pub use crate::meta_storage::{AnyBackendError, MetaStorageError, PsqlMetaError};
    pub use crate::operon::{OperonError, UserError};
    pub use crate::scheduler::SchedulerError;
    pub use crate::storage::{DimState, StorageError};
    pub use crate::ui::UiError;
}

pub mod options {
    pub use crate::meta_storage::{
        MemMetaStorageOptions, MetaBackendOptions, PsqlMetaStorageOptions,
    };
    pub use crate::operon::OperonOptions;
    #[allow(deprecated)]
    pub use crate::storage::{PsqlStorageOptions, StorageOptions};
    pub use crate::ui::UiMode;

    /// A logging level, type alias for `tracing::Level`
    pub type LogLevel = tracing::Level;
}

// Re-export the external crates used in the macro expansions
#[doc(hidden)]
pub mod __private {
    pub use async_trait;
    pub use futures;
    pub use tracing;

    pub use crate::meta_storage::{
        MetaBackend, MetaClientApi, MetaConnApi, MetaResolutionApi, MetaTicketApi, MetaTxApi,
    };
    pub use crate::scheduler::{
        JobHandler, JobRebuilder, JobSpec, PeerEvent, PeerEventSender, PeerEventSenderMap,
        PeerEventSenders, SchedulerHandler, SpecWithMetadata, ValidOperon,
    };
    pub use crate::schema::*;
    pub use crate::storage::psql::{EntityQueries, PsqlStorage};
    pub use crate::utils::{SchemaPrefix, get_dop_coords, get_dop_tags};
}
