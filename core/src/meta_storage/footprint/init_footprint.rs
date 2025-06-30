use crate::meta_storage::{MetaClient, MetaStorageError};

/// Initialize the footprint table.
/// Note that this function is idempotent, i.e. calling it multiple times,
/// or calling it on an already-initialized storage will do nothing.
pub async fn init_footprint(client: MetaClient<'_>) -> Result<(), MetaStorageError> {
    let schema_prefix = client.schema_prefix();

    let stmt = format!(
        "CREATE TABLE IF NOT EXISTS {schema_prefix}footprint (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )"
    );
    client.execute(&stmt, &[]).await?;
    Ok(())
}
