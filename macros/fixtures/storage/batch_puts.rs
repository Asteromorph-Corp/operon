async fn put_all_a(
    &self,
    entity: operon::schema::Entity<0usize, Vec<A>>,
) -> Result<(), operon::storage::StorageError> {
    self.conn()
        .await?
        .entity(self.entities_meta.a)
        .batch_put(entity)
        .await
}

async fn put_all_b(
    &self,
    entity: operon::schema::Entity<1usize, Vec<B>>,
) -> Result<(), operon::storage::StorageError> {
    self.conn()
        .await?
        .entity(self.entities_meta.b)
        .batch_put(entity)
        .await
}

async fn put_all_c(
    &self,
    entity: operon::schema::Entity<1usize, Vec<C>>,
) -> Result<(), operon::storage::StorageError> {
    self.conn()
        .await?
        .entity(self.entities_meta.c)
        .batch_put(entity)
        .await
}
