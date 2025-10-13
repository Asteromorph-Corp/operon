pub async fn get_resolution_l(
    client: operon::meta_storage::MetaClient<'_>,
    j: schema::JDim,
    k: schema::KDim,
) -> Result<Option<schema::LResolution>, operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let stmt = format!("SELECT l_ub FROM {schema_prefix}dimension_l WHERE j = $1 AND k = $2;");
    let Some(row) = client
        .query_opt(&stmt, &[&i64::try_from(j)?, &i64::try_from(k)?])
        .await?
    else {
        return Ok(None);
    };
    Ok(Some(schema::LResolution(
        usize::try_from(row.get::<_, i64>("l_ub"))?,
        j,
        k,
    )))
}
