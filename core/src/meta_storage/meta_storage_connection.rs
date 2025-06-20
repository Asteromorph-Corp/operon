use crate::meta_storage::{MetaClient, MetaStorageError};

type ToSql = dyn ::tokio_postgres::types::ToSql + Sync;

/// Minimal connection information for the metadata storage operations.
#[derive(Debug, Clone, Copy)]
pub struct MetaStorageConnection<'a, Cl>
where
    Cl: MetaClient + 'a,
{
    // Both references live as long as this connection.
    pub client: &'a Cl,
    pub schema: &'a Option<String>,
}

impl<'a, Cl> MetaStorageConnection<'a, Cl>
where
    Cl: MetaClient + 'a,
{
    pub fn new(client: &'a Cl, schema: &'a Option<String>) -> Self {
        Self { client, schema }
    }

    /// Get the schema prefix.
    pub fn get_prefix(&self) -> String {
        match self.schema {
            Some(s) => format!("{s}."),
            None => String::new(),
        }
    }

    pub async fn batch_execute(&self, query: &str) -> Result<(), MetaStorageError> {
        self.client.batch_execute(query).await.map_err(Into::into)
    }

    pub async fn execute(&self, query: &str, params: &[&ToSql]) -> Result<u64, MetaStorageError> {
        self.client.execute(query, params).await.map_err(Into::into)
    }

    pub async fn query(
        &self,
        query: &str,
        params: &[&ToSql],
    ) -> Result<Vec<::tokio_postgres::Row>, MetaStorageError> {
        self.client.query(query, params).await.map_err(Into::into)
    }

    pub async fn query_opt(
        &self,
        query: &str,
        params: &[&ToSql],
    ) -> Result<Option<::tokio_postgres::Row>, MetaStorageError> {
        self.client
            .query_opt(query, params)
            .await
            .map_err(Into::into)
    }

    pub async fn copy_in<T, U>(
        &self,
        query: &T,
    ) -> Result<::tokio_postgres::CopyInSink<U>, MetaStorageError>
    where
        T: ?Sized + ::tokio_postgres::ToStatement + Send + Sync,
        U: ::bytes::Buf + 'static + Send + Sync,
    {
        self.client.copy_in(query).await.map_err(Into::into)
    }
}

impl<'a, Cl> MetaStorageConnection<'a, Cl>
where
    Cl: MetaClient + 'a,
{
    /// If given, initialize the schema in the database.
    pub async fn init_schema(&self) -> Result<(), MetaStorageError> {
        let Some(schema) = self.schema else {
            return Ok(());
        };
        let create_schema = format!("CREATE SCHEMA IF NOT EXISTS {schema}");
        self.client.execute(&create_schema, &[]).await?;
        Ok(())
    }

    pub async fn init_footprints(&self) -> Result<(), MetaStorageError> {
        let schema_prefix = self.get_prefix();
        let stmt = format!(
            "CREATE TABLE IF NOT EXISTS {schema_prefix}footprint (
                        key TEXT PRIMARY KEY,
                        value TEXT NOT NULL
                    )"
        );
        self.execute(&stmt, &[]).await?;
        Ok(())
    }
}
