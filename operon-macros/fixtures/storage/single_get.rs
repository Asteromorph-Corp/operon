async fn get_b(
    &self,
    coordinate: [usize; 2usize],
) -> operon::error::StorageResult<Option<B>, Self::Error> {
    self.conn()
        .await?
        .entity(self.entities_meta.b)
        .get(coordinate)
        .await
}
