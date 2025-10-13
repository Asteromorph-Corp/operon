pub async fn clear_resolution_i(
    client: operon::meta_storage::MetaClient<'_>,
) -> Result<(), operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let stmt = format!("TRUNCATE TABLE {schema_prefix}dimension_i;");
    client.execute(&stmt, &[]).await?;
    Ok(())
}
