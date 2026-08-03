/// Generated trait containing task methods that should be implemented for use with Operon.
///
/// Each task method returns `Self::Error`, the service's error type.
/// `#[derive(OperonService)]` defaults it to `operon::error::UserError`.
/// Select a concrete type with `#[operon(error = MyError)]` on the derive.
///
/// The derive also takes `#[operon(defined_at = "path")]`, the module `define_operon!`
/// expanded in, and `#[operon(crate = "path")]`, the `operon` crate itself.
///
/// # Methods
/// ```rust,ignore
/// async fn alpha(&self) -> Result<Vec<A>, Self::Error>
/// async fn beta(&self, a: A) -> Result<Vec<B>, Self::Error>
/// async fn gamma(&self, a: A) -> Result<Vec<C>, Self::Error>
/// async fn delta(&self, a: A, b: B, c: C) -> Result<D, Self::Error>
/// async fn epsilon(&self, b_j: Vec<B>, d_j: Vec<D>) -> Result<E, Self::Error>
/// async fn zeta(&self, c_k: Vec<C>, e_k: Vec<E>) -> Result<F, Self::Error>
/// ```
pub trait CookingService:
    operon::OperonService<
        JobEnum = schema::JobEnum,
        ResolutionEnum = schema::ResolutionEnum,
        TicketEnum = schema::TicketEnum,
    >
{
    /// ```rust,ignore
    /// async fn alpha(&self) -> Result<Vec<A>, Self::Error>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// A<i> = alpha()
    /// ```
    #[allow(clippy::too_many_arguments)]
    async fn alpha(&self) -> Result<Vec<A>, Self::Error>;

    /// ```rust,ignore
    /// async fn beta(&self, a: A) -> Result<Vec<B>, Self::Error>
    /// ```
    /// Corresponds to the task:
    ///```rust,ignore
    /// B<j> = beta(A) for i
    /// ```
    #[allow(clippy::too_many_arguments)]
    async fn beta(&self, a: A) -> Result<Vec<B>, Self::Error>;

    /// ```rust,ignore
    /// async fn gamma(&self, a: A) -> Result<Vec<C>, Self::Error>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// C<k> = gamma(A) for i
    /// ```
    #[allow(clippy::too_many_arguments)]
    async fn gamma(&self, a: A) -> Result<Vec<C>, Self::Error>;

    /// ```rust,ignore
    /// async fn delta(&self, a: A, b: B, c: C) -> Result<D, Self::Error>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// D = delta(A, B, C) for i, j, k
    /// ```
    #[allow(clippy::too_many_arguments)]
    async fn delta(&self, a: A, b: B, c: C) -> Result<D, Self::Error>;

    /// ```rust,ignore
    /// async fn epsilon(&self, b_j: Vec<B>, d_j: Vec<D>) -> Result<E, Self::Error>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// E = epsilon(B<j>, D<j>) for i, k
    /// ```
    #[allow(clippy::too_many_arguments)]
    async fn epsilon(&self, b_j: Vec<B>, d_j: Vec<D>) -> Result<E, Self::Error>;

    /// ```rust,ignore
    /// async fn zeta(&self, c_k: Vec<C>, e_k: Vec<E>) -> Result<F, Self::Error>
    /// ```
    /// Corresponds to the task:
    /// ```rust,ignore
    /// F = zeta(C<k>, E<k>) for i
    /// ```
    #[allow(clippy::too_many_arguments)]
    async fn zeta(&self, c_k: Vec<C>, e_k: Vec<E>) -> Result<F, Self::Error>;
}
