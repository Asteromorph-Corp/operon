async fn get_a(
    &self,
    coordinate: [usize; 1usize],
) -> operon::error::StorageResult<Option<A>, Self::Error>;

async fn put_a(&self, entity: operon::Entity<1usize, A>)
-> operon::error::StorageResult<(), Self::Error>;

async fn get_b(
    &self,
    coordinate: [usize; 2usize],
) -> operon::error::StorageResult<Option<B>, Self::Error>;

async fn put_b(&self, entity: operon::Entity<2usize, B>)
-> operon::error::StorageResult<(), Self::Error>;

async fn get_c(
    &self,
    coordinate: [usize; 2usize],
) -> operon::error::StorageResult<Option<C>, Self::Error>;

async fn put_c(&self, entity: operon::Entity<2usize, C>)
-> operon::error::StorageResult<(), Self::Error>;

async fn get_d(
    &self,
    coordinate: [usize; 3usize],
) -> operon::error::StorageResult<Option<D>, Self::Error>;

async fn put_d(&self, entity: operon::Entity<3usize, D>)
-> operon::error::StorageResult<(), Self::Error>;

async fn get_e(
    &self,
    coordinate: [usize; 2usize],
) -> operon::error::StorageResult<Option<E>, Self::Error>;

async fn put_e(&self, entity: operon::Entity<2usize, E>)
-> operon::error::StorageResult<(), Self::Error>;

async fn get_f(
    &self,
    coordinate: [usize; 1usize],
) -> operon::error::StorageResult<Option<F>, Self::Error>;

async fn put_f(&self, entity: operon::Entity<1usize, F>)
-> operon::error::StorageResult<(), Self::Error>;
