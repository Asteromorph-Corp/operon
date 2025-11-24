async fn get_all_b_over_j(
    &self,
    [i]: [usize; 1usize],
) -> Result<Vec<B>, operon::storage::StorageError> {
    let conn = self.pool.get().await?;
    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());
    let stmt = format!("SELECT value, j\nFROM {schema_prefix}b\nWHERE i = $1\nORDER BY j");
    let rows = conn.query(&stmt, &[&i64::try_from(i)?]).await?;

    let mut result: Vec<B> = Default::default();
    for row in rows {
        let value = operon::serde_json::from_value::<B>(row.get(0))?;
        result.push(value.into());
    }

    Ok(result)
}

async fn get_all_c_over_k(
    &self,
    [i]: [usize; 1usize],
) -> Result<Vec<C>, operon::storage::StorageError> {
    let conn = self.pool.get().await?;
    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());
    let stmt = format!("SELECT value, k\nFROM {schema_prefix}c\nWHERE i = $1\nORDER BY k");
    let rows = conn.query(&stmt, &[&i64::try_from(i)?]).await?;
    let mut result: Vec<C> = Default::default();
    for row in rows {
        let value = operon::serde_json::from_value::<C>(row.get(0))?;
        result.push(value.into());
    }
    Ok(result)
}

async fn get_all_d_over_j(
    &self,
    [i, k]: [usize; 2usize],
) -> Result<Vec<D>, operon::storage::StorageError> {
    let conn = self.pool.get().await?;
    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());
    let stmt =
        format!("SELECT value, j\nFROM {schema_prefix}d\nWHERE i = $1 AND k = $2\nORDER BY j");
    let rows = conn
        .query(&stmt, &[&i64::try_from(i)?, &i64::try_from(k)?])
        .await?;

    let mut result: Vec<D> = Default::default();
    for row in rows {
        let value = operon::serde_json::from_value::<D>(row.get(0))?;
        result.push(value.into());
    }

    Ok(result)
}

async fn get_all_e_over_k(
    &self,
    [i]: [usize; 1usize],
) -> Result<Vec<E>, operon::storage::StorageError> {
    let conn = self.pool.get().await?;
    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());
    let stmt = format!("SELECT value, k\nFROM {schema_prefix}e\nWHERE i = $1\nORDER BY k");
    let rows = conn.query(&stmt, &[&i64::try_from(i)?]).await?;
    let mut result: Vec<E> = Default::default();
    for row in rows {
        let value = operon::serde_json::from_value::<E>(row.get(0))?;
        result.push(value.into());
    }
    Ok(result)
}
