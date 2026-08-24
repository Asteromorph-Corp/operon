async fn get_b(
    &self,
    coordinate: [usize; 2usize],
) -> operon::error::StorageResult<Option<B>, Self::Error> {
    let b = self.b.get(&coordinate).map(|entry| entry.clone());
    Ok(b)
}
