use crate::meta_storage::{ConnectionWithSchema, MetaStorageError, MetaStorageOptions};

#[derive(Debug, Clone)]
pub struct MetaStorage {
    pub pool: deadpool_postgres::Pool,
    pub schema: Option<String>,
}

impl MetaStorage {
    pub fn new(options: MetaStorageOptions) -> Result<Self, MetaStorageError> {
        let schema = options.schema;

        // Might want to make these hardcoded config values configurable.
        let pg_config: tokio_postgres::Config = Self::build_config(&options.database_uri)?;
        let manager_config = deadpool_postgres::ManagerConfig {
            recycling_method: deadpool_postgres::RecyclingMethod::Clean,
        };
        let manager = deadpool_postgres::Manager::from_config(
            pg_config,
            tokio_postgres::NoTls,
            manager_config,
        );
        let pool = deadpool_postgres::Pool::builder(manager)
            .max_size(options.pool_size)
            .build()?;
        Ok(MetaStorage { pool, schema })
    }

    fn build_config(database_uri: &str) -> Result<tokio_postgres::Config, MetaStorageError> {
        let mut config: ::tokio_postgres::Config = database_uri
            .parse()
            .map_err(|_| MetaStorageError::DatabaseUriParseError(database_uri.into()))?;
        config
            .keepalives(true)
            .keepalives_idle(::std::time::Duration::from_secs(60))
            .keepalives_interval(::std::time::Duration::from_secs(30)); // TODO: Make this configurable.
        Ok(config)
    }

    pub async fn conn(&self) -> Result<ConnectionWithSchema, MetaStorageError> {
        let client = self.pool.get().await?;
        let schema = self.schema.as_deref();

        Ok(ConnectionWithSchema::new(client, schema))
    }

    pub async fn conn_static(&self) -> Result<ConnectionWithSchema<'static>, MetaStorageError> {
        let client = self.pool.get().await?;
        let schema = self.schema.clone();

        Ok(ConnectionWithSchema::new(client, schema))
    }
}
