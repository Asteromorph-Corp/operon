async fn get_a(
    &self,
    coordinate: [usize; 1usize],
) -> Result<Option<A>, operon::storage::StorageError>;

async fn put_a(
    &self,
    entity: operon::schema::Entity<1usize, A>,
) -> Result<(), operon::storage::StorageError>;

async fn get_b(
    &self,
    coordinate: [usize; 2usize],
) -> Result<Option<B>, operon::storage::StorageError>;

async fn put_b(
    &self,
    entity: operon::schema::Entity<2usize, B>,
) -> Result<(), operon::storage::StorageError>;

async fn get_c(
    &self,
    coordinate: [usize; 2usize],
) -> Result<Option<C>, operon::storage::StorageError>;

async fn put_c(
    &self,
    entity: operon::schema::Entity<2usize, C>,
) -> Result<(), operon::storage::StorageError>;

async fn get_d(
    &self,
    coordinate: [usize; 3usize],
) -> Result<Option<D>, operon::storage::StorageError>;

async fn put_d(
    &self,
    entity: operon::schema::Entity<3usize, D>,
) -> Result<(), operon::storage::StorageError>;

async fn get_e(
    &self,
    coordinate: [usize; 2usize],
) -> Result<Option<E>, operon::storage::StorageError>;

async fn put_e(
    &self,
    entity: operon::schema::Entity<2usize, E>,
) -> Result<(), operon::storage::StorageError>;

async fn get_f(
    &self,
    coordinate: [usize; 1usize],
) -> Result<Option<F>, operon::storage::StorageError>;

async fn put_f(
    &self,
    entity: operon::schema::Entity<1usize, F>,
) -> Result<(), operon::storage::StorageError>;
