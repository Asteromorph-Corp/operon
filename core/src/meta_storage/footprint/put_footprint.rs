use crate::meta_storage::{MetaClient, MetaStorageError};

/// Set a footprint key-value pair.
pub async fn put_footprint(
    client: MetaClient<'_>,
    key: &'static str,
    value: String,
) -> Result<(), MetaStorageError> {
    let schema_prefix = client.schema_prefix();

    let stmt = format!(
        "INSERT INTO {schema_prefix}footprint (key, value)
        VALUES ($1, $2)
        ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
    );
    client.execute(&stmt, &[&key, &value]).await?;
    Ok(())
}
