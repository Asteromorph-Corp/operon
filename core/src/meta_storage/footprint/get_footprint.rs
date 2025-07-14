use crate::meta_storage::{MetaClient, MetaStorageError};

/// Get a footprint value by key.
pub async fn get_footprint(
    client: MetaClient<'_>,
    key: &'static str,
) -> Result<Option<String>, MetaStorageError> {
    let schema_prefix = client.schema_prefix();

    let stmt = format!("SELECT value FROM {schema_prefix}footprint WHERE key = $1");
    let row = client.query_opt(&stmt, &[&key]).await?;
    Ok(row.map(|r| r.get::<_, &str>(0).to_string()))
}
