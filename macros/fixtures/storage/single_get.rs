async fn get_b(
    &self,
    i: schema::IDim,
    j: schema::JDim,
) -> Result<Option<B>, operon::storage::StorageError> {
    let conn = self.pool.get().await?;
    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());
    let stmt = format!("SELECT value FROM {schema_prefix}b WHERE i = $1 AND j = $2;");
    let row = conn
        .query_opt(&stmt, &[&i64::try_from(i)?, &i64::try_from(j)?])
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let value = operon::serde_json::from_value::<B_>(row.get(0))?;
    Ok(Some(value.into()))
}
