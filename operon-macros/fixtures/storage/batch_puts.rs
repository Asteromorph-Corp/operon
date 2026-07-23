async fn put_all_a(
    &self,
    entity: operon::Entity<0usize, Vec<A>>,
) -> operon::error::StorageResult<(), Self::Error> {
    self.conn()
        .await?
        .entity(self.entities_meta.a)
        .batch_put(entity)
        .await
}

async fn put_all_b(
    &self,
    entity: operon::Entity<1usize, Vec<B>>,
) -> operon::error::StorageResult<(), Self::Error> {
    self.conn()
        .await?
        .entity(self.entities_meta.b)
        .batch_put(entity)
        .await
}

async fn put_all_c(
    &self,
    entity: operon::Entity<1usize, Vec<C>>,
) -> operon::error::StorageResult<(), Self::Error> {
    self.conn()
        .await?
        .entity(self.entities_meta.c)
        .batch_put(entity)
        .await
}
