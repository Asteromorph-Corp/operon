async fn get_a(&self, i: schema::IDim) -> Result<Option<A>, operon::storage::StorageError>;

async fn put_a(&self, i: schema::IDim, value: A) -> Result<(), operon::storage::StorageError>;

async fn get_b(
    &self,
    i: schema::IDim,
    j: schema::JDim,
) -> Result<Option<B>, operon::storage::StorageError>;

async fn put_b(
    &self,
    i: schema::IDim,
    j: schema::JDim,
    value: B,
) -> Result<(), operon::storage::StorageError>;

async fn get_c(
    &self,
    i: schema::IDim,
    k: schema::KDim,
) -> Result<Option<C>, operon::storage::StorageError>;

async fn put_c(
    &self,
    i: schema::IDim,
    k: schema::KDim,
    value: C,
) -> Result<(), operon::storage::StorageError>;

async fn get_d(
    &self,
    i: schema::IDim,
    j: schema::JDim,
    k: schema::KDim,
) -> Result<Option<D>, operon::storage::StorageError>;

async fn put_d(
    &self,
    i: schema::IDim,
    j: schema::JDim,
    k: schema::KDim,
    value: D,
) -> Result<(), operon::storage::StorageError>;

async fn get_e(
    &self,
    i: schema::IDim,
    k: schema::KDim,
) -> Result<Option<E>, operon::storage::StorageError>;

async fn put_e(
    &self,
    i: schema::IDim,
    k: schema::KDim,
    value: E,
) -> Result<(), operon::storage::StorageError>;

async fn get_f(&self, i: schema::IDim) -> Result<Option<F>, operon::storage::StorageError>;
async fn put_f(&self, i: schema::IDim, value: F) -> Result<(), operon::storage::StorageError>;
