use std::str::FromStr;
use std::time::Duration;

use secrecy::ExposeSecret;

use crate::meta_storage::meta_client::ConnectionWithSchema;
use crate::meta_storage::{MetaStorageError, MetaStorageOptions};

#[derive(Debug, Clone)]
pub struct MetaStorage {
    pub worker_pool: deadpool_postgres::Pool,
    pub scheduler_pool: deadpool_postgres::Pool,
    pub schema: Option<String>,
}

impl MetaStorage {
    pub fn new(options: MetaStorageOptions) -> Result<Self, MetaStorageError> {
        let mk_pool = |size| {
            create_pool(
                options.database_uri.expose_secret(),
                options.keepalives_idle,
                options.keepalives_interval,
                size,
            )
        };

        let (worker_pool, scheduler_pool) = match options.pool_size {
            0 => return Err(MetaStorageError::PoolSizeTooSmall(0)),
            1 => {
                let p = mk_pool(1)?;
                (p.clone(), p)
            }
            n @ ..=5 => (mk_pool(n - 1)?, mk_pool(1)?),
            n => (mk_pool(n - 5)?, mk_pool(5)?),
        };
        let schema = options.schema;

        Ok(MetaStorage {
            worker_pool,
            scheduler_pool,
            schema,
        })
    }

    pub async fn worker_conn(&self) -> Result<ConnectionWithSchema<'_>, MetaStorageError> {
        let client = self.worker_pool.get().await?;
        let schema = self.schema.as_deref();

        Ok(ConnectionWithSchema::new(client, schema))
    }

    pub async fn scheduler_conn(&self) -> Result<ConnectionWithSchema<'static>, MetaStorageError> {
        let client = self.scheduler_pool.get().await?;
        let schema = self.schema.clone();

        Ok(ConnectionWithSchema::new(client, schema))
    }
}

fn create_pool(
    uri: &str,
    keepalives_idle: Duration,
    keepalives_interval: Duration,
    pool_size: usize,
) -> Result<deadpool_postgres::Pool, MetaStorageError> {
    // Might want to make these hardcoded config values configurable.
    let mut pg_config = tokio_postgres::Config::from_str(uri)?;
    pg_config
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
