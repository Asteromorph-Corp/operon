use crate::meta_storage::{MetaClient, MetaStorageError};

/// Clear the footprint table.
pub async fn clear_footprint(client: MetaClient<'_>) -> Result<(), MetaStorageError> {
    let schema_prefix = client.schema_prefix();

    let stmt = format!("TRUNCATE TABLE {schema_prefix}footprint");
    client.execute(&stmt, &[]).await?;
    Ok(())
}
