//! The backend-agnostic metadata storage API.
//!
//! The concrete metadata store lives behind the [`MetaBackend`] trait and its associated API
//! traits.
//! A backend is selected at runtime via [`MetaBackendOptions`](super::MetaBackendOptions) and
//! monomorphized at compile time: [`Operon::run`](crate::Operon::run) matches the options enum once
//! and dispatches into a generic inner parameterized by a concrete `MSto: MetaBackend`.

use uuid::Uuid;

use crate::meta_storage::MetaStorageError;
use crate::schema::{
    DimensionMetadata, Job, JobMetadata, Resolution, RunFootprint, Ticket, TicketStatus,
};

/// A concrete metadata storage backend.
///
/// Exposes the connection/handle machinery (`Conn`/`Tx`/`Client`) and the ticket/resolution query
/// builders as associated types.
/// The handle is stored by value and cloned across `tokio::spawn`, hence the `Clone + Send + Sync +
/// 'static` bounds.
pub trait MetaBackend: Clone + std::fmt::Debug + Send + Sync + Sized + 'static {
    /// The backend-specific options this backend is constructed from.
    type Options;

    /// A checked-out connection. Borrows the backend for `worker_conn`, `'static` for
    /// `scheduler_conn`.
    type Conn<'a>: MetaConnApi<Self> + Send
    where
        Self: 'a;

    /// A transaction opened over a [`Conn`](Self::Conn).
    type Tx<'a>: MetaTxApi<Self> + Send
    where
        Self: 'a;

    /// A cheap, `Copy` handle for issuing queries, borrowed from a `Conn` or a `Tx`.
    type Client<'a>: MetaClientApi<Self> + Copy + Send + Sync
    where
        Self: 'a;

    /// The ticket query builder for a job of arity `N`, borrowed from a `Client`.
    type Ticket<'a, const N: usize>: MetaTicketApi<N> + Send
    where
        Self: 'a;

    /// The resolution query builder for a dimension of arity `N`, borrowed from a `Client`.
    type Resolution<'a, const N: usize>: MetaResolutionApi<N> + Send
    where
        Self: 'a;

    /// Builds the backend from its options.
    fn new(options: Self::Options) -> Result<Self, MetaStorageError>;

    /// Checks out a connection for a spawned worker.
    fn worker_conn(&self) -> impl Future<Output = Result<Self::Conn<'_>, MetaStorageError>> + Send;

    /// Checks out a connection for synchronous (individual-)scheduler operations.
    fn scheduler_conn(
        &self,
    ) -> impl Future<Output = Result<Self::Conn<'static>, MetaStorageError>> + Send;

    /// Acquires the exclusive lock guarding this run's metadata against a second Operon instance.
    ///
    /// Lazy and idempotent; forced once at scheduler startup.
    fn ensure_lock(&self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;

    /// Re-checks that the lock acquired by [`ensure_lock`](Self::ensure_lock) is still held.
    fn check_lock(&self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
}

/// A checked-out metadata connection.
pub trait MetaConnApi<MSto: MetaBackend>: Send {
    /// Begins a transaction over this connection.
    fn transaction(
        &mut self,
    ) -> impl Future<Output = Result<MSto::Tx<'_>, MetaStorageError>> + Send;

    /// Borrows this connection as a client for issuing queries.
    fn as_client(&self) -> MSto::Client<'_>;
}

/// A metadata transaction.
pub trait MetaTxApi<MSto: MetaBackend>: Send {
    /// Commits the transaction.
    fn commit(self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;

    /// Rolls the transaction back.
    fn rollback(self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;

    /// Borrows this transaction as a client for issuing queries.
    fn as_client(&self) -> MSto::Client<'_>;
}

/// A handle for issuing metadata queries, in terms of the domain types.
pub trait MetaClientApi<MSto: MetaBackend>: Copy + Send + Sync {
    /// Builds ticket queries for the given job.
    fn ticket<const N: usize>(&self, job_meta: JobMetadata<N>) -> MSto::Ticket<'_, N>;

    /// Builds resolution queries for the given dimension.
    fn resolution<const N: usize>(&self, dim_meta: DimensionMetadata<N>)
    -> MSto::Resolution<'_, N>;

    // --- Schema initialization ---

    fn init_schema(&self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
    fn init_ticket_hash(&self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
    fn init_dimension_hash(&self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
    fn init_ticket_status_type(&self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
    fn init_ticket_summary(&self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;

    // --- Footprint ---

    fn init_footprint(&self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
    fn clear_footprint(&self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
    fn get_footprint(
        &self,
    ) -> impl Future<Output = Result<Option<RunFootprint>, MetaStorageError>> + Send;
    fn upsert_run(
        &self,
        footprint: &RunFootprint,
    ) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
    fn put_execution(
        &self,
        run_id: Uuid,
        execution_id: Uuid,
    ) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
    fn update_execution_on_finish(
        &self,
        footprint: &RunFootprint,
        execution_id: Uuid,
    ) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
}

/// Ticket operations for a job of arity `N`.
pub trait MetaTicketApi<const N: usize> {
    fn init(&self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
    fn clear(&self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
    fn get_all(
        &self,
        status: TicketStatus,
    ) -> impl Future<Output = Result<Vec<Ticket<N>>, MetaStorageError>> + Send;
    fn put(&self, ticket: Ticket<N>) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
    fn raise_deps_done<const M: usize>(
        &self,
        upstream_meta: JobMetadata<M>,
        upstream_job: Job<M>,
        aggregate_dims: &[&'static str],
    ) -> impl Future<Output = Result<Vec<Ticket<N>>, MetaStorageError>> + Send;
    fn raise_deps_quota<const M: usize>(
        &self,
        upstream_meta: JobMetadata<M>,
        upstream_ticket: Ticket<M>,
        aggregate_dims: &[&'static str],
        ub: usize,
    ) -> impl Future<Output = Result<Vec<Ticket<N>>, MetaStorageError>> + Send;
    fn explode<const M: usize, const IDX: usize>(
        &self,
        res_meta: DimensionMetadata<M>,
        res: Resolution<M>,
    ) -> impl Future<Output = Result<Vec<Ticket<N>>, MetaStorageError>> + Send;
    fn mark_done(&self, job: Job<N>) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
    fn get_status(&self) -> impl Future<Output = Result<(i64, i64, i64), MetaStorageError>> + Send;
}

/// Resolution operations for a dimension of arity `N`.
pub trait MetaResolutionApi<const N: usize> {
    fn init(&self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
    fn clear(&self) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
    fn get(
        &self,
        coordinate: [usize; N],
    ) -> impl Future<Output = Result<Option<Resolution<N>>, MetaStorageError>> + Send;
    fn put(
        &self,
        resolution: Resolution<N>,
    ) -> impl Future<Output = Result<(), MetaStorageError>> + Send;
}
