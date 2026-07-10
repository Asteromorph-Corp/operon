use crate::meta_storage::psql::PsqlMetaStorageOptions;

/// Selects and parameterizes the metadata storage backend.
///
/// This is the forward-looking, backend-agnostic way to say *where* the metadata lives. Each
/// variant owns its backend's full parameter set (e.g. [`Psql`](MetaBackendOptions::Psql) carries a
/// [`PsqlMetaStorageOptions`]), so backend-specific knobs never leak into backend-agnostic options.
///
/// # Stability
///
/// This enum is `#[non_exhaustive]`: adding a backend variant must stay a non-breaking change.
/// Prefer the provided constructors (e.g. [`psql`]) over naming variants directly.
///
/// Note that the *convenience* entry points that assume Postgres — notably
/// [`OperonOptions::new`](crate::options::OperonOptions::new) — are **not** covered by this
/// guarantee. Once a second backend exists, those will be replaced by backend-specific
/// constructors (e.g. `OperonOptions::from_psql_uri`), which *is* a breaking change. Code that
/// wants to be forward-compatible should go through this enum via
/// [`OperonOptions::from_backend`](crate::options::OperonOptions::from_backend).
///
/// [`psql`]: MetaBackendOptions::psql
#[non_exhaustive]
pub enum MetaBackendOptions {
    /// The Postgres backend and its configuration.
    Psql(PsqlMetaStorageOptions),
}

impl MetaBackendOptions {
    /// Selects the Postgres backend with the given connection URI and default tuning.
    ///
    /// For finer control, build a [`PsqlMetaStorageOptions`] and wrap it in
    /// [`MetaBackendOptions::Psql`] directly.
    pub fn psql(uri: impl Into<String>) -> Self {
        MetaBackendOptions::Psql(PsqlMetaStorageOptions::new(uri))
    }
}
