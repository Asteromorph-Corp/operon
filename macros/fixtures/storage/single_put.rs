async fn put_b(
    &self,
    entity: operon::schema::Entity<2usize, B>,
) -> Result<(), operon::storage::StorageError> {
    self.conn()
        .await?
        .entity(self.entities_meta.b)
        .put(entity)
        .await
}
