pub trait CookingService:
    operon::service::OperonService<JobEnum = schema::JobEnum, ResolutionEnum = schema::ResolutionEnum>
{
    async fn alpha(&self) -> Result<Vec<A>, operon::operon::UserError>;
    async fn beta(&self, a: A) -> Result<Vec<B>, operon::operon::UserError>;
    async fn gamma(&self, a: A) -> Result<Vec<C>, operon::operon::UserError>;
    async fn delta(&self, a: A, b: B, c: C) -> Result<D, operon::operon::UserError>;
    async fn epsilon(&self, b_j: Vec<B>, d_j: Vec<D>) -> Result<E, operon::operon::UserError>;
    async fn zeta(&self, c_k: Vec<C>, e_k: Vec<E>) -> Result<F, operon::operon::UserError>;
}
