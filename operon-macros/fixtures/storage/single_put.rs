async fn put_b(
    &self,
    entity: operon::Entity<2usize, B>,
) -> Result<(), operon::error::StorageError> {
    self.conn()
        .await?
        .entity(self.entities_meta.b)
        .put(entity)
        .await
}
