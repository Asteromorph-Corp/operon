use crate::meta_storage::{MetaClient, MetaStorageError};

/// If given, initialize the schema in the database.
pub async fn init_schema(client: MetaClient<'_>) -> Result<(), MetaStorageError> {
    let Some(schema) = client.schema() else {
        return Ok(());
    };
    let create_schema = format!("CREATE SCHEMA IF NOT EXISTS {schema}");
    client.execute(&create_schema, &[]).await?;
    Ok(())
}
