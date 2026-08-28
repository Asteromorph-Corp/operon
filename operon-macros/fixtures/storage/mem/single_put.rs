async fn put_b(
    &self,
    entity: operon::Entity<2usize, B>,
) -> operon::error::StorageResult<(), Self::Error> {
    self.b.insert(entity.coordinate, entity.value);
    Ok(())
}
