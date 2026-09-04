/// ```rust,ignore
/// async fn get_a(&self, coordinate: [usize; 1]) -> StorageResult<Option<A>, Self::Error>
/// ```
/// Reads the `A` stored at `[i]`, or `None` if that coordinate holds nothing.
async fn get_a(
    &self,
    coordinate: [usize; 1usize],
) -> operon::error::StorageResult<Option<A>, Self::Error>;

/// ```rust,ignore
/// async fn put_a(&self, entity: Entity<1, A>) -> StorageResult<(), Self::Error>
/// ```
/// Writes the given `A` at its own coordinate `[i]`, replacing whatever is stored there.
async fn put_a(
    &self,
    entity: operon::Entity<1usize, A>,
) -> operon::error::StorageResult<(), Self::Error>;
