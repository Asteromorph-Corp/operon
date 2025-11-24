async fn put_b(&self, i: usize, j: usize, value: B) -> Result<(), operon::storage::StorageError> {
    let entity = operon::schema::Entity {
        coordinate: [i, j],
        value,
    };
    self.conn()
        .await?
        .entity(self.entities_meta.b)
        .put(entity)
        .await
}
