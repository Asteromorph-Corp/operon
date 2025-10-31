pub trait CookingService:
    operon::service::OperonService<JobEnum = schema::JobEnum, ResolutionEnum = schema::ResolutionEnum>
{
    async fn alpha(&self) -> Result<Vec<A>, Box<dyn std::error::Error + Send + Sync>>;
    async fn beta(&self, a: A) -> Result<Vec<B>, Box<dyn std::error::Error + Send + Sync>>;
    async fn gamma(&self, a: A) -> Result<Vec<C>, Box<dyn std::error::Error + Send + Sync>>;
    async fn delta(&self, a: A, b: B, c: C) -> Result<D, Box<dyn std::error::Error + Send + Sync>>;
    async fn epsilon(
        &self,
        b_j: Vec<B>,
        d_j: Vec<D>,
    ) -> Result<E, Box<dyn std::error::Error + Send + Sync>>;
    async fn zeta(
        &self,
        c_k: Vec<C>,
        e_k: Vec<E>,
    ) -> Result<F, Box<dyn std::error::Error + Send + Sync>>;
}
