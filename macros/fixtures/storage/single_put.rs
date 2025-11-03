async fn put_b(
    &self,
    i: schema::IDim,
    j: schema::JDim,
    value: B,
) -> Result<(), operon::storage::StorageError> {
    let conn = self.pool.get().await?;
    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());
    let stmt = format!(
        "INSERT INTO {schema_prefix}b (i, j, value)
VALUES ($1, $2, $3)
ON CONFLICT (i, j) DO UPDATE SET value = EXCLUDED.value;"
    );
    let value: B_ = value.into();
    conn.execute(
        &stmt,
        &[
            &i64::try_from(i)?,
            &i64::try_from(j)?,
            &operon::serde_json::to_value(value)?,
        ],
    )
    .await?;
    Ok(())
}
