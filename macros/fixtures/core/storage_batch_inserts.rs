async fn put_all_a(
    &self,
    entity: operon::schema::Entity<0usize, Vec<A>>,
) -> Result<(), operon::storage::StorageError> {
    let [] = entity.coordinate;
    for (i, value) in entity.value.into_iter().enumerate() {
        let entity_single = operon::schema::Entity {
            coordinate: [i],
            value,
        };
        self.put_a(entity_single).await?;
    }
    Ok(())
}

async fn put_all_b(
    &self,
    entity: operon::schema::Entity<1usize, Vec<B>>,
) -> Result<(), operon::storage::StorageError> {
    let [i] = entity.coordinate;
    for (j, value) in entity.value.into_iter().enumerate() {
        let entity_single = operon::schema::Entity {
            coordinate: [i, j],
            value,
        };
        self.put_b(entity_single).await?;
    }
    Ok(())
}

async fn put_all_c(
    &self,
    entity: operon::schema::Entity<1usize, Vec<C>>,
) -> Result<(), operon::storage::StorageError> {
    let [i] = entity.coordinate;
    for (k, value) in entity.value.into_iter().enumerate() {
        let entity_single = operon::schema::Entity {
            coordinate: [i, k],
            value,
        };
        self.put_c(entity_single).await?;
    }
    Ok(())
}
