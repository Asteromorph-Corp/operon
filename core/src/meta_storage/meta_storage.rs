use crate::meta_storage::{MetaClient, MetaStorageError};

pub trait MetaStorage: Send + Sync {
    fn get_client(&self) -> &MetaClient<'_>;
    fn get_schema(&self) -> Option<&str>;
    fn get_prefix(&self) -> String {
        match self.get_schema() {
            Some(s) => format!("{s}."),
            None => String::new(),
        }
    }

    fn get_client_owned(self) -> MetaClient<'static>;
    async fn commit(self) -> Result<(), MetaStorageError>
    where
        Self: Sized,
    {
        match self.get_client_owned() {
            MetaClient::Object(client) => client.batch_execute("COMMIT").await?,
            MetaClient::Transaction(tx) => tx.commit().await?,
        }
        Ok(())
    }

    /// If given, initialize the schema in the database.
    async fn init_schema(&self) -> Result<(), MetaStorageError> {
        let Some(schema) = self.get_schema() else {
            return Ok(());
        };
        let create_schema = format!("CREATE SCHEMA IF NOT EXISTS {schema}");
        self.get_client().execute(&create_schema, &[]).await?;
        Ok(())
    }

    async fn init_footprints(&self) -> Result<(), MetaStorageError> {
        let schema_prefix = self.get_prefix();
        let stmt = format!(
            "CREATE TABLE IF NOT EXISTS {schema_prefix}footprint (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )"
        );
        self.get_client().execute(&stmt, &[]).await?;
        Ok(())
    }

    type Resolution: Send + Sync;
    type ResolutionRequest: Default + Send + Sync;

    async fn init_resolution(&self) -> Result<(), MetaStorageError>;

    async fn get_resolution(
        &self,
        request: &Self::ResolutionRequest,
    ) -> Result<Option<Self::Resolution>, MetaStorageError>;

    async fn init_tickets(&self) -> Result<(), MetaStorageError>;

    async fn get_footprint(
        &self,
        key: impl Into<String>,
    ) -> Result<Option<String>, MetaStorageError> {
        let key: String = key.into();
        let schema_prefix = self.get_prefix();
        let stmt = format!("SELECT value FROM {schema_prefix}footprint WHERE key = $1");
        let row = self.get_client().query_opt(&stmt, &[&key]).await?;
        Ok(row.map(|r| r.get::<_, &str>(0).to_string()))
    }
}

// pub struct MetaStorage {
//     pub pool: deadpool_postgres::Pool,
//     pub schema: Option<String>,
// }

// impl MetaStorage {
//     fn build_config(database_uri: &str) -> Result<tokio_postgres::Config, MetaStorageError> {
//         let mut config: ::tokio_postgres::Config = database_uri
//             .parse()
//             .map_err(|_| MetaStorageError::DatabaseUriParseError(database_uri.into()))?;
//         config
//             .keepalives(true)
//             .keepalives_idle(::std::time::Duration::from_secs(60))
//             .keepalives_interval(::std::time::Duration::from_secs(30)); // TODO: Make this configurable.
//         Ok(config)
//     }

//     pub fn new(options: MetaStorageOptions) -> Result<Self, MetaStorageError> {
//         let schema = options.schema;

//         // Might want to make these hardcoded config values configurable.
//         let pg_config: tokio_postgres::Config = Self::build_config(&options.database_uri)?;
//         let manager_config = deadpool_postgres::ManagerConfig {
//             recycling_method: deadpool_postgres::RecyclingMethod::Clean,
//         };
//         let manager = deadpool_postgres::Manager::from_config(
//             pg_config,
//             tokio_postgres::NoTls,
//             manager_config,
//         );
//         let pool = deadpool_postgres::Pool::builder(manager)
//             .max_size(options.pool_size)
//             .build()?;

//         Ok(Self { pool, schema })
//     }

//     pub async fn init<Sto: OperonStorage, Svc>(
//         &self,
//         promoter: &impl Promoter<Sto, Svc>,
//     ) -> Result<(), MetaStorageError> {
//         // Initialize the metadata storage.
//         let mut client = self.pool.get().await?;
//         let tx = client.transaction().await?;
//         let conn = MetaStorageConnection::new(&tx, &self.schema);
//         conn.init_schema().await?;
//         conn.init_footprints().await?;
//         promoter.init_facts(&conn).await?;
//         promoter.init_tickets(&conn).await?;
//         tx.commit().await?;
//         Ok(())
//     }

// }
