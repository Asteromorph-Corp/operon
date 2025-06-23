use crate::meta_storage::{
    MetaStorage, MetaStorageConnector, MetaStorageError, MetaStorageOptions,
};

pub struct MetaContext<C: MetaStorageConnector> {
    pub pool: deadpool_postgres::Pool,
    pub schema: Option<String>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C: MetaStorageConnector> MetaContext<C> {
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

        let ctx = MetaContext {
            pool,
            schema,
            _phantom: std::marker::PhantomData,
        };

        ctx.init().await?;

        Ok(ctx)
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

    async fn init(&self) -> Result<(), MetaStorageError> {
        // Initialize the metadata storage.
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;
        let conn = C::connect(tx, self.schema.as_deref());
        conn.init_schema().await?;
        conn.init_footprints().await?;
        conn.init_resolution().await?;
        conn.init_tickets().await?;
        conn.commit().await?;
        Ok(())
    }

    pub async fn get_conn(&self) -> Result<C::MetaSto<'_>, MetaStorageError> {
        let client = self.pool.get().await?;
        Ok(C::connect(client, self.schema.as_deref()))
    }
}
