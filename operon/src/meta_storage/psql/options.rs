use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use tokio::sync::OnceCell;

use crate::meta_storage::psql::error::PsqlResult;
use crate::meta_storage::psql::{PsqlMetaError, PsqlMetaStorage};

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

    /// Builds the Postgres metadata backend from these options.
    pub fn build(self) -> PsqlResult<PsqlMetaStorage> {
        let PsqlMetaStorageOptions {
            uri,
            pool_size,
            schema,
            keepalives_idle,
            keepalives_interval,
        } = self;

        let mk_pool = |size| {
            create_pool(
                uri.expose_secret(),
                keepalives_idle,
                keepalives_interval,
                size,
            )
        };

        // The user specifies how many connections we may open to the metadata database.
        // We require at least two, and split the connections into three tiers:
        // 1. One single connection for the advisory lock on the schema.
        // 2. One to five connections for synchronous (individual-)scheduler operations.
        // 3. The remaining connections for spawned workers.
        let (worker_pool, scheduler_pool, lock_pool) = match pool_size {
            0..=1 => return Err(PsqlMetaError::PoolSizeTooSmall(pool_size).into()),
            2 => {
                let p = mk_pool(1)?;
                (p.clone(), p, mk_pool(1)?)
            }
            n @ ..=6 => (mk_pool(n - 2)?, mk_pool(1)?, mk_pool(1)?),
            n => (mk_pool(n - 6)?, mk_pool(5)?, mk_pool(1)?),
        };

        Ok(PsqlMetaStorage {
            worker_pool,
            scheduler_pool,
            schema,
            lock_pool,
            lock: Arc::new(OnceCell::new()),
        })
    }
}

fn create_pool(
    uri: &str,
    keepalives_idle: Duration,
    keepalives_interval: Duration,
    pool_size: usize,
) -> Result<deadpool_postgres::Pool, PsqlMetaError> {
    // Might want to make these hardcoded config values configurable.
    let mut pg_config = tokio_postgres::Config::from_str(uri)?;
    let _ = pg_config
        .keepalives(true)
        .keepalives_idle(keepalives_idle)
        .keepalives_interval(keepalives_interval);
    let manager_config = deadpool_postgres::ManagerConfig {
        recycling_method: deadpool_postgres::RecyclingMethod::Clean,
    };
    let manager =
        deadpool_postgres::Manager::from_config(pg_config, tokio_postgres::NoTls, manager_config);
    let pool = deadpool_postgres::Pool::builder(manager)
        .max_size(pool_size)
        .build()?;
    Ok(pool)
}
