use secrecy::SecretString;

/// Configuration for the Postgres entity-storage backend.
pub struct PsqlStorageOptions {
    pub database_uri: SecretString,
    pub pool_size: usize,
    pub keepalives_idle: std::time::Duration,
    pub keepalives_interval: std::time::Duration,
    pub schema: Option<String>,
}

/// Former name of [`PsqlStorageOptions`], kept as an alias for backward compatibility.
#[deprecated(note = "renamed to `PsqlStorageOptions`")]
pub type StorageOptions = PsqlStorageOptions;

impl PsqlStorageOptions {
    pub fn new(database_uri: impl Into<String>) -> Self {
        Self {
            database_uri: SecretString::from(database_uri.into()),
            pool_size: 16,
            keepalives_idle: std::time::Duration::from_secs(60),
            keepalives_interval: std::time::Duration::from_secs(30),
            schema: None,
        }
    }

    pub fn with_pool_size(mut self, size: usize) -> Self {
        self.pool_size = size;
        self
    }

    pub fn with_keepalives_idle(mut self, duration: std::time::Duration) -> Self {
        self.keepalives_idle = duration;
        self
    }

    pub fn with_keepalives_interval(mut self, duration: std::time::Duration) -> Self {
        self.keepalives_interval = duration;
        self
    }

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
}
