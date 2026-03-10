use crate::storage::StorageError;
use crate::storage::psql::StorageClient;

impl<'a> StorageClient<'a> {
    /// Initializes the data storage schema, if specified.
    pub async fn init_schema(&self) -> Result<(), StorageError> {
        let Some(schema) = self.schema() else {
            return Ok(());
        };
        let create_schema = format!("CREATE SCHEMA IF NOT EXISTS {schema}");
        self.execute(&create_schema, &[]).await?;
        Ok(())
    }

    /// Initializes the entity hash table.
    pub async fn init_entity_hash(&self) -> Result<(), StorageError> {
        let schema = self.schema_prefix();

        let create_table = format!(
            "CREATE TABLE IF NOT EXISTS {schema}_entity_hash (
                id TEXT PRIMARY KEY,
                hash TEXT NOT NULL
            )",
        );
        self.execute(&create_table, &[]).await?;
        Ok(())
    }
}
