use crate::storage::psql::PsqlStorageResult;
use crate::storage::psql::client::StorageClient;
use crate::storage::psql::queries::entity::ENTITY_SHAPES;
use crate::utils::init_shape_table_query;

impl StorageClient<'_> {
    /// Initializes the data storage schema, if specified.
    pub async fn init_schema(&self) -> PsqlStorageResult<()> {
        let Some(schema) = self.schema() else {
            return Ok(());
        };
        let create_schema = format!("CREATE SCHEMA IF NOT EXISTS {schema}");
        let _num_rows = self.execute(&create_schema, &[]).await?;
        Ok(())
    }

    /// Initializes the table recording each entity's shape.
    pub async fn init_entity_hash(&self) -> PsqlStorageResult<()> {
        let create_table = init_shape_table_query(ENTITY_SHAPES, self.schema_prefix());
        let _num_rows = self.execute(&create_table, &[]).await?;
        Ok(())
    }
}
