async fn get_b(
    &self,
    coordinate: [usize; 2usize],
) -> Result<Option<B>, operon::error::StorageError> {
    self.conn()
        .await?
        .entity(self.entities_meta.b)
        .get(coordinate)
        .await
}
