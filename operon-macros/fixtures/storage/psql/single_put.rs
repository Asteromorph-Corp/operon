async fn put_b(
    &self,
    entity: operon::Entity<2usize, B>,
) -> operon::error::StorageResult<(), Self::Error> {
    self.conn()
        .await?
        .entity(self.entities_meta.b)
        .put(entity)
        .await
}
