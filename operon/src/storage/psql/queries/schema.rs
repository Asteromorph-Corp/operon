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
}
