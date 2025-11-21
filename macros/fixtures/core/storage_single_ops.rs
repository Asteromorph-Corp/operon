async fn get_a(&self, i: usize) -> Result<Option<A>, operon::storage::StorageError>;

async fn put_a(&self, i: usize, value: A) -> Result<(), operon::storage::StorageError>;

async fn get_b(&self, i: usize, j: usize) -> Result<Option<B>, operon::storage::StorageError>;

async fn put_b(&self, i: usize, j: usize, value: B) -> Result<(), operon::storage::StorageError>;

async fn get_c(&self, i: usize, k: usize) -> Result<Option<C>, operon::storage::StorageError>;

async fn put_c(&self, i: usize, k: usize, value: C) -> Result<(), operon::storage::StorageError>;

async fn get_d(
    &self,
    i: usize,
    j: usize,
    k: usize,
) -> Result<Option<D>, operon::storage::StorageError>;

async fn put_d(
    &self,
    i: usize,
    j: usize,
    k: usize,
    value: D,
) -> Result<(), operon::storage::StorageError>;

async fn get_e(&self, i: usize, k: usize) -> Result<Option<E>, operon::storage::StorageError>;

async fn put_e(&self, i: usize, k: usize, value: E) -> Result<(), operon::storage::StorageError>;

async fn get_f(&self, i: usize) -> Result<Option<F>, operon::storage::StorageError>;
async fn put_f(&self, i: usize, value: F) -> Result<(), operon::storage::StorageError>;
