use std::str::FromStr;
use std::time::Duration;

use secrecy::ExposeSecret;

use crate::meta_storage::meta_client::ConnectionWithSchema;
use crate::meta_storage::{MetaStorageError, MetaStorageOptions};

#[derive(Debug, Clone)]
pub struct MetaStorage {
    pub pool: deadpool_postgres::Pool,
    pub ui_pool: deadpool_postgres::Pool,
    pub schema: Option<String>,
}

impl MetaStorage {
    pub fn new(options: MetaStorageOptions) -> Result<Self, MetaStorageError> {
        let pool = create_pool(
            options.database_uri.expose_secret(),
            options.keepalives_idle,
            options.keepalives_interval,
            options.pool_size,
        )?;
        let ui_pool = create_pool(
            options.database_uri.expose_secret(),
            options.keepalives_idle,
            options.keepalives_interval,
            5,
        )?;
        let schema = options.schema;

        Ok(MetaStorage {
            pool,
            ui_pool,
            schema,
        })
    }

    pub async fn conn(&self) -> Result<ConnectionWithSchema<'_>, MetaStorageError> {
        let client = self.pool.get().await?;
        let schema = self.schema.as_deref();

        Ok(ConnectionWithSchema::new(client, schema))
    }

    pub async fn conn_static(&self) -> Result<ConnectionWithSchema<'static>, MetaStorageError> {
        let client = self.pool.get().await?;
        let schema = self.schema.clone();

        Ok(ConnectionWithSchema::new(client, schema))
    }

    pub async fn ui_conn(&self) -> Result<ConnectionWithSchema<'static>, MetaStorageError> {
        let client = self.ui_pool.get().await?;
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
