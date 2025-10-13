pub async fn init_resolution_i(
    client: operon::meta_storage::MetaClient<'_>,
) -> Result<(), operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let stmt = format!(
        "CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_i (
    i_ub BIGINT NOT NULL
);"
    );
    client.execute(&stmt, &[]).await?;
    Ok(())
}
