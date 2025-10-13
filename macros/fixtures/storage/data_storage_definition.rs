#[derive(Debug, Clone)]
pub struct PsqlCookingStorage<A_, B_, C_, D_, E_, F_> {
    pub pool: operon::deadpool_postgres::Pool,
    pub schema: Option<String>,
    _phantom: std::marker::PhantomData<(A_, B_, C_, D_, E_, F_)>,
}
