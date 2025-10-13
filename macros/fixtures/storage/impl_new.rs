impl<A_, B_, C_, D_, E_, F_> PsqlCookingStorage<A_, B_, C_, D_, E_, F_> {
    pub fn new(
        options: operon::storage::StorageOptions,
    ) -> Result<Self, operon::storage::StorageError> {
        let pg_config: operon::tokio_postgres::Config = {
            let mut config = operon::secrecy::ExposeSecret::expose_secret(&options.database_uri)
                .parse::<operon::tokio_postgres::Config>()
                .map_err(|e| operon::storage::StorageError::DatabaseUriParseError(e.to_string()))?;
            config
                .keepalives(true)
                .keepalives_idle(options.keepalives_idle)
                .keepalives_interval(options.keepalives_interval);
            config
        };
        let manager_config = operon::deadpool_postgres::ManagerConfig {
            recycling_method: operon::deadpool_postgres::RecyclingMethod::Clean,
        };
        let manager = operon::deadpool_postgres::Manager::from_config(
            pg_config,
            operon::tokio_postgres::NoTls,
            manager_config,
        );
        let pool = operon::deadpool_postgres::Pool::builder(manager)
            .max_size(options.pool_size)
            .build()?;

        Ok(Self {
            pool,
            schema: options.schema,
            _phantom: std::marker::PhantomData,
        })
    }
}
