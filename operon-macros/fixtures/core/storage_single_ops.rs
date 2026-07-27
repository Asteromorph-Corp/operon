/// ```rust,ignore
/// async fn get_a(coordinate: [usize; 1]) -> StorageResult<Option<A>, Self::Error>
/// ```
/// Reads the `A` stored at `[i]`, or `None` if that coordinate holds nothing.
async fn get_a(
    &self,
    coordinate: [usize; 1usize],
) -> operon::error::StorageResult<Option<A>, Self::Error>;

/// ```rust,ignore
/// async fn put_a(entity: Entity<1, A>) -> StorageResult<(), Self::Error>
/// ```
/// Writes the given `A` at its own coordinate `[i]`, replacing whatever is stored there.
async fn put_a(&self, entity: operon::Entity<1usize, A>)
-> operon::error::StorageResult<(), Self::Error>;

/// ```rust,ignore
/// async fn get_b(coordinate: [usize; 2]) -> StorageResult<Option<B>, Self::Error>
/// ```
/// Reads the `B` stored at `[i, j]`, or `None` if that coordinate holds nothing.
async fn get_b(
    &self,
    coordinate: [usize; 2usize],
) -> operon::error::StorageResult<Option<B>, Self::Error>;

/// ```rust,ignore
/// async fn put_b(entity: Entity<2, B>) -> StorageResult<(), Self::Error>
/// ```
/// Writes the given `B` at its own coordinate `[i, j]`, replacing whatever is stored there.
async fn put_b(&self, entity: operon::Entity<2usize, B>)
-> operon::error::StorageResult<(), Self::Error>;

/// ```rust,ignore
/// async fn get_c(coordinate: [usize; 2]) -> StorageResult<Option<C>, Self::Error>
/// ```
/// Reads the `C` stored at `[i, k]`, or `None` if that coordinate holds nothing.
async fn get_c(
    &self,
    coordinate: [usize; 2usize],
) -> operon::error::StorageResult<Option<C>, Self::Error>;

/// ```rust,ignore
/// async fn put_c(entity: Entity<2, C>) -> StorageResult<(), Self::Error>
/// ```
/// Writes the given `C` at its own coordinate `[i, k]`, replacing whatever is stored there.
async fn put_c(&self, entity: operon::Entity<2usize, C>)
-> operon::error::StorageResult<(), Self::Error>;

/// ```rust,ignore
/// async fn get_d(coordinate: [usize; 3]) -> StorageResult<Option<D>, Self::Error>
/// ```
/// Reads the `D` stored at `[i, j, k]`, or `None` if that coordinate holds nothing.
async fn get_d(
    &self,
    coordinate: [usize; 3usize],
) -> operon::error::StorageResult<Option<D>, Self::Error>;

/// ```rust,ignore
/// async fn put_d(entity: Entity<3, D>) -> StorageResult<(), Self::Error>
/// ```
/// Writes the given `D` at its own coordinate `[i, j, k]`, replacing whatever is stored there.
async fn put_d(&self, entity: operon::Entity<3usize, D>)
-> operon::error::StorageResult<(), Self::Error>;

/// ```rust,ignore
/// async fn get_e(coordinate: [usize; 2]) -> StorageResult<Option<E>, Self::Error>
/// ```
/// Reads the `E` stored at `[i, k]`, or `None` if that coordinate holds nothing.
async fn get_e(
    &self,
    coordinate: [usize; 2usize],
) -> operon::error::StorageResult<Option<E>, Self::Error>;

/// ```rust,ignore
/// async fn put_e(entity: Entity<2, E>) -> StorageResult<(), Self::Error>
/// ```
/// Writes the given `E` at its own coordinate `[i, k]`, replacing whatever is stored there.
async fn put_e(&self, entity: operon::Entity<2usize, E>)
-> operon::error::StorageResult<(), Self::Error>;

/// ```rust,ignore
/// async fn get_f(coordinate: [usize; 1]) -> StorageResult<Option<F>, Self::Error>
/// ```
/// Reads the `F` stored at `[i]`, or `None` if that coordinate holds nothing.
async fn get_f(
    &self,
    coordinate: [usize; 1usize],
) -> operon::error::StorageResult<Option<F>, Self::Error>;

/// ```rust,ignore
/// async fn put_f(entity: Entity<1, F>) -> StorageResult<(), Self::Error>
/// ```
/// Writes the given `F` at its own coordinate `[i]`, replacing whatever is stored there.
async fn put_f(&self, entity: operon::Entity<1usize, F>)
-> operon::error::StorageResult<(), Self::Error>;
