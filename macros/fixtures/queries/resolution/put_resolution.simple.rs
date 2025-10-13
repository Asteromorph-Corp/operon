pub async fn put_resolution_i(
    client: operon::meta_storage::MetaClient<'_>,
    resolution: &schema::IResolution,
) -> Result<(), operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let stmt = format!(
        "INSERT INTO {schema_prefix}dimension_i (i_ub) VALUES ($1) ON CONFLICT DO NOTHING;"
    );
    client
        .execute(&stmt, &[&i64::try_from(resolution.0)?])
        .await?;
    Ok(())
}
