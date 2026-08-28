use secrecy::SecretString;

use crate::storage::StorageError;
use crate::storage::psql::{FromPsqlStorageOptions, PsqlStorageError};

/// Configuration for the Postgres entity-storage backend.
pub struct PsqlStorageOptions {
    pub database_uri: SecretString,
    pub pool_size: usize,
    pub keepalives_idle: std::time::Duration,
    pub keepalives_interval: std::time::Duration,
    pub schema: Option<String>,
}

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

    /// Builds the Postgres entity storage for a pipeline from these options.
    ///
    /// Turbofish or name the generated storage type to select the pipeline:
    ///
    /// ```rust,ignore
    /// let psql_storage_options = PsqlStorageOptions::new("postgresql://user:pass@localhost/db")
    ///     .with_schema("my_data_schema")
    ///     .with_pool_size(32);
    /// let storage = psql_storage_options.build::<PsqlCookingStorage>()?;
    /// let storage: PsqlCookingStorage = psql_storage_options.build()?;
    /// ```
    pub fn build<S: FromPsqlStorageOptions>(self) -> Result<S, StorageError<PsqlStorageError>> {
        S::from_options(self)
    }
}
