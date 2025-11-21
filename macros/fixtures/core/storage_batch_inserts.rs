async fn put_all_a(&self, values: Vec<A>) -> Result<(), operon::storage::StorageError> {
    for (i, value) in values.into_iter().enumerate() {
        self.put_a(i.into(), value).await?;
    }
    Ok(())
}

async fn put_all_b(&self, i: usize, values: Vec<B>) -> Result<(), operon::storage::StorageError> {
    for (j, value) in values.into_iter().enumerate() {
        self.put_b(i, j.into(), value).await?;
    }
    Ok(())
}

async fn put_all_c(&self, i: usize, values: Vec<C>) -> Result<(), operon::storage::StorageError> {
    for (k, value) in values.into_iter().enumerate() {
        self.put_c(i, k.into(), value).await?;
    }
    Ok(())
}
