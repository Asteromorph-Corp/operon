pub async fn get_resolution_i(
    client: operon::meta_storage::MetaClient<'_>,
) -> Result<Option<schema::IResolution>, operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let stmt = format!("SELECT i_ub FROM {schema_prefix}dimension_i;");
    let Some(row) = client.query_opt(&stmt, &[]).await? else {
        return Ok(None);
    };
    Ok(Some(schema::IResolution(usize::try_from(
        row.get::<_, i64>("i_ub"),
    )?)))
}
