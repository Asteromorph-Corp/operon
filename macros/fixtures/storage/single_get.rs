async fn get_b(&self, i: usize, j: usize) -> Result<Option<B>, operon::storage::StorageError> {
    let entity = self
        .conn()
        .await?
        .entity(self.entities_meta.b)
        .get([i, j])
        .await?;
    Ok(entity.map(|e| e.value))
}
