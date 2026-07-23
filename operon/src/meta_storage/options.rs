use crate::meta_storage::mem::MemMetaStorageOptions;
use crate::meta_storage::psql::PsqlMetaStorageOptions;
use crate::meta_storage::{AnyBackend, AnyBackendError, MetaStorageError};

/// Selects and parameterizes the metadata storage backend.
///
/// This is the forward-looking, backend-agnostic way to say *where* the metadata lives. Each
/// variant owns its backend's full parameter set (e.g. [`Psql`](MetaBackendOptions::Psql) carries a
/// [`PsqlMetaStorageOptions`]), so backend-specific knobs never leak into backend-agnostic options.
///
/// Build the selected backend into an [`AnyBackend`] with [`build`](Self::build), then hand it to
/// [`Operon::new`](crate::Operon::new).
/// To pin a backend at compile time, build its concrete options directly (e.g.
/// [`PsqlMetaStorageOptions::build`]).
///
/// # Stability
///
/// This enum is `#[non_exhaustive]`: adding a backend variant must stay a non-breaking change.
/// Reach it through the provided constructors (e.g. [`psql`]) or a [`From`] conversion rather than
/// naming variants directly.
///
/// [`psql`]: MetaBackendOptions::psql
#[non_exhaustive]
pub enum MetaBackendOptions {
    /// The Postgres backend and its configuration.
    Psql(PsqlMetaStorageOptions),
    /// The in-memory backend and its configuration.
    Mem(MemMetaStorageOptions),
}

impl MetaBackendOptions {
    /// Selects the Postgres backend with the given connection URI and default tuning.
    ///
    /// For finer control, build a [`PsqlMetaStorageOptions`] and convert it with
    /// [`Into`]/[`From`].
    pub fn psql(uri: impl Into<String>) -> Self {
        MetaBackendOptions::Psql(PsqlMetaStorageOptions::new(uri))
    }

    /// Selects the in-memory backend.
    ///
    /// The metadata lives in process and is lost when it exits.
    pub fn mem() -> Self {
        MetaBackendOptions::Mem(MemMetaStorageOptions::new())
    }

    /// Builds the metadata backend selected by these options into an [`AnyBackend`].
    ///
    /// The concrete backend is resolved at runtime from the chosen variant.
    /// To pin a backend at compile time, build its concrete options instead
    /// (e.g. [`PsqlMetaStorageOptions::build`]).
    pub fn build(self) -> Result<AnyBackend, MetaStorageError<AnyBackendError>> {
        match self {
            MetaBackendOptions::Psql(options) => Ok(AnyBackend::Psql(options.build()?)),
            MetaBackendOptions::Mem(options) => Ok(AnyBackend::Mem(options.build())),
        }
    }
}

impl From<PsqlMetaStorageOptions> for MetaBackendOptions {
    fn from(options: PsqlMetaStorageOptions) -> Self {
        MetaBackendOptions::Psql(options)
    }
}

impl From<MemMetaStorageOptions> for MetaBackendOptions {
    fn from(options: MemMetaStorageOptions) -> Self {
        MetaBackendOptions::Mem(options)
    }
}
