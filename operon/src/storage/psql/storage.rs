use async_trait::async_trait;
use secrecy::ExposeSecret;

use crate::schema::RunFootprint;
use crate::storage::psql::client::StorageClient;
use crate::storage::psql::{EntityQueries, PsqlStorageError, PsqlStorageResult};
use crate::storage::{OperonStorage, PsqlStorageOptions};
use crate::utils::SchemaPrefix;

/// The SQL storage that can be used with the service.
///
/// This can only be used when all entities implement `Serialize` and `DeserializeOwned`.
#[derive(Debug, Clone)]
pub struct PsqlStorage<T> {
    pub pool: deadpool_postgres::Pool,
    pub schema: Option<String>,
    pub entities_meta: T,
}

impl<T> PsqlStorage<T> {
    pub async fn conn(&self) -> PsqlStorageResult<StorageClient<'_>> {
        let client = self.pool.get().await?;
        let schema = self.schema.as_deref();

        Ok(StorageClient::new(client, schema))
    }

    pub fn schema_prefix(&self) -> SchemaPrefix<'_> {
        SchemaPrefix(self.schema.as_deref())
    }
}

/// Constructs a concrete [`PsqlStorage`] alias from [`PsqlStorageOptions`].
///
/// This lets [`PsqlStorageOptions::build`](crate::options::PsqlStorageOptions::build) be
/// turbofished on the generated storage alias, e.g. `build::<PsqlCookingStorage>()`.
pub trait FromPsqlStorageOptions: Sized {
    fn from_options(options: PsqlStorageOptions) -> PsqlStorageResult<Self>;
}

impl<T: Default> FromPsqlStorageOptions for PsqlStorage<T> {
    fn from_options(options: PsqlStorageOptions) -> PsqlStorageResult<Self> {
        PsqlStorage::<T>::new(options)
    }
}

impl<T: Default> PsqlStorage<T> {
    pub(crate) fn new(options: PsqlStorageOptions) -> PsqlStorageResult<Self> {
        let pg_config: tokio_postgres::Config = {
            let mut config = options
                .database_uri
                .expose_secret()
                .parse::<tokio_postgres::Config>()?;
            config
                .keepalives(true)
                .keepalives_idle(options.keepalives_idle)
                .keepalives_interval(options.keepalives_interval);
            config
        };
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

        Ok(Self {
            pool,
            schema: options.schema,
            entities_meta: T::default(),
        })
    }
}

#[async_trait]
impl<T: EntityQueries> OperonStorage for PsqlStorage<T> {
    type Error = PsqlStorageError;

    /// If the storage schema is specified, initialize the schema in the database.
    async fn init(&self) -> PsqlStorageResult<()> {
        let client = self.conn().await?;
        let entities_init_stmt = self.entities_meta.init_stmt(self.schema_prefix());

        client.init_schema().await?;
        client.init_entity_hash().await?;
        client.init_footprint().await?;
        client.batch_execute(&entities_init_stmt).await?;
        Ok(())
    }

    async fn clear(&self) -> PsqlStorageResult<()> {
        let client = self.conn().await?;
        let entities_clear_stmt = self.entities_meta.clear_stmt(self.schema_prefix());

        client.clear_footprint().await?;
        client.batch_execute(&entities_clear_stmt).await?;
        Ok(())
    }

    async fn get_footprint(&self) -> PsqlStorageResult<Option<RunFootprint>> {
        let client = self.conn().await?;
        client.get_footprint().await
    }

    async fn put_footprint(&self, footprint: &RunFootprint) -> PsqlStorageResult<()> {
        let client = self.conn().await?;
        client.put_footprint(footprint).await
    }

    async fn clear_footprint(&self) -> PsqlStorageResult<()> {
        let client = self.conn().await?;
        client.clear_footprint().await
    }
}
