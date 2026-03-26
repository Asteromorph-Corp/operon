async fn get_a(
    &self,
    coordinate: [usize; 1usize],
) -> Result<Option<A>, operon::error::StorageError>;

async fn put_a(&self, entity: operon::Entity<1usize, A>)
-> Result<(), operon::error::StorageError>;

async fn get_b(
    &self,
    coordinate: [usize; 2usize],
) -> Result<Option<B>, operon::error::StorageError>;

async fn put_b(&self, entity: operon::Entity<2usize, B>)
-> Result<(), operon::error::StorageError>;

async fn get_c(
    &self,
    coordinate: [usize; 2usize],
) -> Result<Option<C>, operon::error::StorageError>;

async fn put_c(&self, entity: operon::Entity<2usize, C>)
-> Result<(), operon::error::StorageError>;

async fn get_d(
    &self,
    coordinate: [usize; 3usize],
) -> Result<Option<D>, operon::error::StorageError>;

async fn put_d(&self, entity: operon::Entity<3usize, D>)
-> Result<(), operon::error::StorageError>;

async fn get_e(
    &self,
    coordinate: [usize; 2usize],
) -> Result<Option<E>, operon::error::StorageError>;

async fn put_e(&self, entity: operon::Entity<2usize, E>)
-> Result<(), operon::error::StorageError>;

async fn get_f(
    &self,
    coordinate: [usize; 1usize],
) -> Result<Option<F>, operon::error::StorageError>;

async fn put_f(&self, entity: operon::Entity<1usize, F>)
-> Result<(), operon::error::StorageError>;
