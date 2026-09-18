/// ```rust,ignore
/// async fn put_all_a(&self, entity: Entity<0, Vec<A>>) -> StorageResult<(), Self::Error>
/// ```
/// Writes a whole run of `A` at `[i]`, taking `i` from each value's position in `entity.value`.
/// Defaults to walking `put_a` one entity at a time.
async fn put_all_a(
    &self,
    entity: operon::Entity<0usize, Vec<A>>,
) -> operon::error::StorageResult<(), Self::Error> {
    let [] = entity.coordinate;
    for (i, value) in entity.value.into_iter().enumerate() {
        let entity_single = operon::Entity {
            coordinate: [i],
            value,
        };
        self.put_a(entity_single).await?;
    }
    Ok(())
}
