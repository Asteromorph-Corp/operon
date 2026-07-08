use secrecy::SecretString;

/// Selects and parameterizes the metadata storage backend.
///
/// This is the forward-looking, backend-agnostic way to say *where* the metadata lives.
///
/// # Stability
///
/// This enum is `#[non_exhaustive]`: adding a backend variant must stay a non-breaking
/// change.
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
    /// The Postgres backend, parameterized by its connection URI.
    Psql(SecretString),
}

impl MetaBackendOptions {
    /// Selects the Postgres backend with the given connection URI.
    pub fn psql(uri: impl Into<String>) -> Self {
        MetaBackendOptions::Psql(SecretString::from(uri.into()))
    }
}

pub struct MetaStorageOptions {
    pub(crate) backend: MetaBackendOptions,
    pub(crate) pool_size: usize,
    pub(crate) keepalives_idle: std::time::Duration,
    pub(crate) keepalives_interval: std::time::Duration,
    pub(crate) schema: Option<String>,
}
