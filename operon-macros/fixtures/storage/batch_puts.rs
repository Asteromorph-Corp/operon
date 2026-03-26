async fn put_all_a(
    &self,
    entity: operon::Entity<0usize, Vec<A>>,
) -> Result<(), operon::error::StorageError> {
    self.conn()
        .await?
        .entity(self.entities_meta.a)
        .batch_put(entity)
        .await
}

async fn put_all_b(
    &self,
    entity: operon::Entity<1usize, Vec<B>>,
) -> Result<(), operon::error::StorageError> {
    self.conn()
        .await?
        .entity(self.entities_meta.b)
        .batch_put(entity)
        .await
}

async fn put_all_c(
    &self,
    entity: operon::Entity<1usize, Vec<C>>,
) -> Result<(), operon::error::StorageError> {
    self.conn()
        .await?
        .entity(self.entities_meta.c)
        .batch_put(entity)
        .await
}
