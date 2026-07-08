use std::time::Duration;

use secrecy::SecretString;

/// Configuration for the Postgres metadata backend.
///
/// Carries every knob specific to Postgres: the connection URI, the connection-pool size, the
/// target schema, and the TCP keepalive tuning.
///
/// Construct via [`PsqlMetaStorageOptions::new`] (which fills sensible defaults) and tune with the
/// `with_*` builders, or reach it through
/// [`MetaBackendOptions::psql`](super::super::MetaBackendOptions::psql).
#[derive(Debug, Clone)]
pub struct PsqlMetaStorageOptions {
    pub(crate) uri: SecretString,
    pub(crate) pool_size: usize,
    pub(crate) schema: Option<String>,
    pub(crate) keepalives_idle: Duration,
    pub(crate) keepalives_interval: Duration,
}

impl PsqlMetaStorageOptions {
    /// Creates Postgres options for the given connection URI, with defaults for everything else.
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: SecretString::from(uri.into()),
            pool_size: 16,
            schema: Some("metadata".to_string()),
            keepalives_idle: Duration::from_secs(60),
            keepalives_interval: Duration::from_secs(30),
        }
    }

    /// Sets the maximum number of connections in the metadata connection pool.
    pub fn with_pool_size(mut self, size: u32) -> Self {
        self.pool_size = size as usize;
        self
    }

    /// Sets the schema the metadata tables live in.
    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        let schema = schema.into();

        if !schema.chars().all(|c| c.is_alphanumeric() || c == '_') {
            panic!("Schema name can only contain alphanumeric characters and underscores");
        } else if schema.is_empty() {
            panic!("Schema name cannot be empty");
        }

        self.schema = Some(schema);
        self
    }

    /// Sets the TCP keepalive idle duration for metadata connections.
    pub fn with_keepalives_idle(mut self, duration: Duration) -> Self {
        self.keepalives_idle = duration;
        self
    }

    /// Sets the TCP keepalive interval for metadata connections.
    pub fn with_keepalives_interval(mut self, duration: Duration) -> Self {
        self.keepalives_interval = duration;
        self
    }
}
