pub async fn put_resolution_j(
    client: operon::meta_storage::MetaClient<'_>,
    resolution: &schema::JResolution,
) -> Result<(), operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let stmt = format!(
        "INSERT INTO {schema_prefix}dimension_j (i, j_ub) VALUES ($1, $2) ON CONFLICT DO NOTHING;"
    );
    client
        .execute(
            &stmt,
            &[&i64::try_from(resolution.1)?, &i64::try_from(resolution.0)?],
        )
        .await?;
    Ok(())
}
