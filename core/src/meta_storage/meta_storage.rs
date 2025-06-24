use async_trait::async_trait;
use deadpool_postgres::Object;

use crate::{
    dimension::Resolution,
    meta_storage::{MetaClient, MetaContext, MetaStorageError},
    ui::UiStateUpdate,
};

#[async_trait]
pub trait MetaStorage: Send + Sync + 'static {
    fn new(context: MetaContext) -> Self;

    async fn client(&self) -> Result<Object, MetaStorageError>;
    fn schema(&self) -> Option<&str>;

    fn get_prefix(&self) -> String {
        match self.schema() {
            Some(s) => format!("{s}."),
            None => String::new(),
        }
    }

    async fn init(&self) -> Result<(), MetaStorageError> {
        let mut client = self.client().await?;
        let tx = client.transaction().await?;
        self.init_schema(&tx).await?;
        self.init_footprint(&tx).await?;
        self.init_resolution(&tx).await?;
        self.init_tickets(&tx).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn init_schema(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError> {
        let Some(schema) = self.schema() else {
            return Ok(());
        };
        let conn: MetaClient<'_> = conn.into();
        let create_schema = format!("CREATE SCHEMA IF NOT EXISTS {schema}");
        conn.execute(&create_schema, &[]).await?;
        Ok(())
    }

    async fn init_footprint(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError> {
        let conn: MetaClient<'_> = conn.into();
        let schema_prefix = self.get_prefix();
        let stmt = format!(
            "CREATE TABLE IF NOT EXISTS {schema_prefix}footprint (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )"
        );
        conn.execute(&stmt, &[]).await?;
        Ok(())
    }

    async fn clear_footprint(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError> {
        let conn: MetaClient<'_> = conn.into();
        let schema_prefix = self.get_prefix();
        let stmt = format!("TRUNCATE TABLE {schema_prefix}footprint");
        conn.execute(&stmt, &[]).await?;
        Ok(())
    }

    async fn get_footprint(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
        key: impl Into<String> + Send + Sync,
    ) -> Result<Option<String>, MetaStorageError> {
        let conn: MetaClient<'_> = conn.into();
        let key: String = key.into();
        let schema_prefix = self.get_prefix();
        let stmt = format!("SELECT value FROM {schema_prefix}footprint WHERE key = $1");
        let row = conn.query_opt(&stmt, &[&key]).await?;
        Ok(row.map(|r| r.get::<_, &str>(0).to_string()))
    }

    async fn put_footprint(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
        key: impl Into<String> + Send + Sync,
        value: impl Into<String> + Send + Sync,
    ) -> Result<(), MetaStorageError> {
        let conn: MetaClient<'_> = conn.into();
        let key: String = key.into();
        let value: String = value.into();

        let schema_prefix = self.get_prefix();
        let stmt = format!(
            "INSERT INTO {schema_prefix}footprint (key, value)
            VALUES ($1, $2)
            ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
        );
        conn.execute(&stmt, &[&key, &value]).await?;
        Ok(())
    }

    type Resolution: Resolution;
    type ResolutionRequest: Default + Send + Sync;

    async fn init_resolution(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError>;

    async fn clear_resolution(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError>;

    async fn get_resolution(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
        request: &Self::ResolutionRequest,
    ) -> Result<Option<Self::Resolution>, MetaStorageError>;

    async fn put_resolution(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
        resolution: &Self::Resolution,
    ) -> Result<(), MetaStorageError>;

    async fn init_tickets(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError>;

    async fn clear_tickets(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError>;

    async fn get_ui_updates(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<Vec<UiStateUpdate>, MetaStorageError>;
}
