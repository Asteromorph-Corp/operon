use crate::meta_storage::{MetaStorageError, MetaStorageOptions};

pub struct MetaContext {
    pub pool: deadpool_postgres::Pool,
    pub schema: Option<String>,
}

impl MetaContext {
    pub async fn new(options: MetaStorageOptions) -> Result<Self, MetaStorageError> {
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
        Ok(MetaContext { pool, schema })
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
}
