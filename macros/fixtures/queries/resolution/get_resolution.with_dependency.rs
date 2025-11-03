pub async fn get_resolution_j(
    client: operon::meta_storage::MetaClient<'_>,
    i: schema::IDim,
) -> Result<Option<schema::JResolution>, operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let stmt = format!("SELECT j_ub FROM {schema_prefix}dimension_j WHERE i = $1;");
    let Some(row) = client.query_opt(&stmt, &[&i64::try_from(i)?]).await? else {
        return Ok(None);
    };
    Ok(Some(schema::JResolution(
        usize::try_from(row.get::<_, i64>("j_ub"))?,
        i,
    )))
}
