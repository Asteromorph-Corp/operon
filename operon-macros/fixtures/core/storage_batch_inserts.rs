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

/// ```rust,ignore
/// async fn put_all_b(&self, entity: Entity<1, Vec<B>>) -> StorageResult<(), Self::Error>
/// ```
/// Writes a whole run of `B` at `[i, j]`, taking `j` from each value's position in `entity.value`.
/// Defaults to walking `put_b` one entity at a time.
async fn put_all_b(
    &self,
    entity: operon::Entity<1usize, Vec<B>>,
) -> operon::error::StorageResult<(), Self::Error> {
    let [i] = entity.coordinate;
    for (j, value) in entity.value.into_iter().enumerate() {
        let entity_single = operon::Entity {
            coordinate: [i, j],
            value,
        };
        self.put_b(entity_single).await?;
    }
    Ok(())
}

/// ```rust,ignore
/// async fn put_all_c(&self, entity: Entity<1, Vec<C>>) -> StorageResult<(), Self::Error>
/// ```
/// Writes a whole run of `C` at `[i, k]`, taking `k` from each value's position in `entity.value`.
/// Defaults to walking `put_c` one entity at a time.
async fn put_all_c(
    &self,
    entity: operon::Entity<1usize, Vec<C>>,
) -> operon::error::StorageResult<(), Self::Error> {
    let [i] = entity.coordinate;
    for (k, value) in entity.value.into_iter().enumerate() {
        let entity_single = operon::Entity {
            coordinate: [i, k],
            value,
        };
        self.put_c(entity_single).await?;
    }
    Ok(())
}
