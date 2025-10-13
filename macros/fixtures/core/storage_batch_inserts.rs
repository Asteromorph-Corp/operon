async fn put_all_b(
    &self,
    i: schema::IDim,
    values: Vec<B>,
) -> Result<(), operon::storage::StorageError> {
    for (j, value) in values.into_iter().enumerate() {
        self.put_b(i, j.into(), value).await?;
    }
    Ok(())
}

async fn put_all_c(
    &self,
    i: schema::IDim,
    values: Vec<C>,
) -> Result<(), operon::storage::StorageError> {
    for (k, value) in values.into_iter().enumerate() {
        self.put_c(i, k.into(), value).await?;
    }
    Ok(())
}
