pub trait CookingService:
    operon::OperonService<
        JobEnum = schema::JobEnum,
        ResolutionEnum = schema::ResolutionEnum,
        TicketEnum = schema::TicketEnum,
    >
{
    /// ```rust,ignore
    /// async fn alpha() -> Result<Vec<A>, operon::error::UserError>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// A<i> = alpha()
    /// ```
    #[allow(clippy::too_many_arguments)]
    async fn alpha(&self) -> Result<Vec<A>, operon::error::UserError>;

    /// ```rust,ignore
    /// async fn beta(a: A) -> Result<Vec<B>, operon::error::UserError>
    /// ```
    /// Corresponds to the task:
    ///```rust,ignore
    /// B<j> = beta(A) for i
    /// ```
    #[allow(clippy::too_many_arguments)]
    async fn beta(&self, a: A) -> Result<Vec<B>, operon::error::UserError>;

    /// ```rust,ignore
    /// async fn gamma(a: A) -> Result<Vec<C>, operon::error::UserError>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// C<k> = gamma(A) for i
    /// ```
    #[allow(clippy::too_many_arguments)]
    async fn gamma(&self, a: A) -> Result<Vec<C>, operon::error::UserError>;

    /// ```rust,ignore
    /// async fn delta(a: A, b: B, c: C) -> Result<D, operon::error::UserError>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// D = delta(A, B, C) for i, j, k
    /// ```
    #[allow(clippy::too_many_arguments)]
    async fn delta(&self, a: A, b: B, c: C) -> Result<D, operon::error::UserError>;

    /// ```rust,ignore
    /// async fn epsilon(b_j: Vec<B>, d_j: Vec<D>) -> Result<E, operon::error::UserError>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// E = epsilon(B<j>, D<j>) for i, k
    /// ```
    #[allow(clippy::too_many_arguments)]
    async fn epsilon(&self, b_j: Vec<B>, d_j: Vec<D>) -> Result<E, operon::error::UserError>;

    /// ```rust,ignore
    /// async fn zeta(c_k: Vec<C>, e_k: Vec<E>) -> Result<F, operon::error::UserError>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// F = zeta(C<k>, E<k>) for i
    /// ```
    #[allow(clippy::too_many_arguments)]
    async fn zeta(&self, c_k: Vec<C>, e_k: Vec<E>) -> Result<F, operon::error::UserError>;
}
