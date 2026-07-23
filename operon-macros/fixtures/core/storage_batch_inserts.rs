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
