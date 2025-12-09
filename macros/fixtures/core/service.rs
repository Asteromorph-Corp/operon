pub trait CookingService:
    operon::service::OperonService<JobEnum = schema::JobEnum, ResolutionEnum = schema::ResolutionEnum>
{
    /// ```rust,ignore
    /// async fn alpha() -> Result<Vec<A>, operon::operon::UserError>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// A<i> = alpha()
    /// ```
    async fn alpha(&self) -> Result<Vec<A>, operon::operon::UserError>;

    /// ```rust,ignore
    /// async fn beta(a: A) -> Result<Vec<B>, operon::operon::UserError>
    /// ```
    /// Corresponds to the task:
    ///```rust,ignore
    /// B<j> = beta(A) for i
    /// ```
    async fn beta(&self, a: A) -> Result<Vec<B>, operon::operon::UserError>;

    /// ```rust,ignore
    /// async fn gamma(a: A) -> Result<Vec<C>, operon::operon::UserError>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// C<k> = gamma(A) for i
    /// ```
    async fn gamma(&self, a: A) -> Result<Vec<C>, operon::operon::UserError>;

    /// ```rust,ignore
    /// async fn delta(a: A, b: B, c: C) -> Result<D, operon::operon::UserError>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// D = delta(A, B, C) for i, j, k
    /// ```
    async fn delta(&self, a: A, b: B, c: C) -> Result<D, operon::operon::UserError>;

    /// ```rust,ignore
    /// async fn epsilon(b_j: Vec<B>, d_j: Vec<D>) -> Result<E, operon::operon::UserError>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// E = epsilon(B<j>, D<j>) for i, k
    /// ```
    async fn epsilon(&self, b_j: Vec<B>, d_j: Vec<D>) -> Result<E, operon::operon::UserError>;

    /// ```rust,ignore
    /// async fn zeta(c_k: Vec<C>, e_k: Vec<E>) -> Result<F, operon::operon::UserError>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// F = zeta(C<k>, E<k>) for i
    /// ```
    async fn zeta(&self, c_k: Vec<C>, e_k: Vec<E>) -> Result<F, operon::operon::UserError>;
}
