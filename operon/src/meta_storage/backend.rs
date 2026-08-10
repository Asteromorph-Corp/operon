//! The backend-agnostic metadata storage API.
//!
//! The concrete metadata store lives behind the [`MetaBackend`] trait and its associated API
//! traits.
//! A program either pins a concrete backend as [`Operon`](crate::Operon)'s `MSto` parameter, or
//! defaults to [`AnyBackend`](super::AnyBackend), which selects the backend at runtime from
//! [`MetaBackendOptions`](super::MetaBackendOptions) and match-dispatches each operation.

use uuid::Uuid;

use crate::meta_storage::MetaResult;
use crate::schema::{
    DimensionMetadata, Job, Resolution, RunFootprint, TableShape, TaskMetadata, Ticket,
    TicketStatus,
};

/// A concrete metadata storage backend.
///
/// Exposes the connection/handle machinery (`Conn`/`Tx`/`Client`) and the ticket/resolution query
/// builders as associated types.
/// The handle is stored by value and cloned across `tokio::spawn`, hence the `Clone + Send + Sync +
/// 'static` bounds.
pub trait MetaBackend: Clone + std::fmt::Debug + Send + Sync + Sized + 'static {
    /// This backend's own error type, surfaced through
    /// [`MetaStorageError::Backend`](crate::meta_storage::MetaStorageError::Backend).
    type Error: std::error::Error + Send + Sync + 'static;

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

    /// The ticket query builder for a task of arity `N`, borrowed from a `Client`.
    type Ticket<'a, const N: usize>: MetaTicketApi<N, Error = Self::Error> + Send
    where
        Self: 'a;

    /// The resolution query builder for a dimension of arity `N`, borrowed from a `Client`.
    type Resolution<'a, const N: usize>: MetaResolutionApi<N, Error = Self::Error> + Send
    where
        Self: 'a;

    /// Checks out a connection for a spawned worker.
    fn worker_conn(&self) -> impl Future<Output = MetaResult<Self::Conn<'_>, Self::Error>> + Send;

    /// Checks out a connection for synchronous (individual-)scheduler operations.
    fn scheduler_conn(
        &self,
    ) -> impl Future<Output = MetaResult<Self::Conn<'static>, Self::Error>> + Send;

    /// Acquires the exclusive lock guarding this run's metadata against a second Operon instance.
    ///
    /// Lazy and idempotent; forced once at scheduler startup.
    fn ensure_lock(&self) -> impl Future<Output = MetaResult<(), Self::Error>> + Send;

    /// Re-checks that the lock acquired by [`ensure_lock`](Self::ensure_lock) is still held.
    fn check_lock(&self) -> impl Future<Output = MetaResult<(), Self::Error>> + Send;
}

/// A checked-out metadata connection.
pub trait MetaConnApi<MSto: MetaBackend>: Send {
    /// Begins a transaction over this connection.
    fn transaction(&mut self)
    -> impl Future<Output = MetaResult<MSto::Tx<'_>, MSto::Error>> + Send;

    /// Borrows this connection as a client for issuing queries.
    fn as_client(&self) -> MSto::Client<'_>;
}

/// A metadata transaction.
pub trait MetaTxApi<MSto: MetaBackend>: Send {
    /// Commits the transaction.
    fn commit(self) -> impl Future<Output = MetaResult<(), MSto::Error>> + Send;

    /// Rolls the transaction back.
    fn rollback(self) -> impl Future<Output = MetaResult<(), MSto::Error>> + Send;

    /// Borrows this transaction as a client for issuing queries.
    fn as_client(&self) -> MSto::Client<'_>;
}

/// A handle for issuing metadata queries, in terms of the domain types.
pub trait MetaClientApi<MSto: MetaBackend>: Copy + Send + Sync {
    /// Builds ticket queries for the given task.
    fn ticket<const N: usize>(&self, task_meta: TaskMetadata<N>) -> MSto::Ticket<'_, N>;

    /// Builds resolution queries for the given dimension.
    fn resolution<const N: usize>(&self, dim_meta: DimensionMetadata<N>)
    -> MSto::Resolution<'_, N>;

    // --- Schema initialization ---
    //
    // Each of these runs as the scheduler starts, so all of them tolerate a store that already
    // holds what they prepare.

    /// Prepares the namespace the metadata lives in.
    fn init_schema(&self) -> impl Future<Output = MetaResult<(), MSto::Error>> + Send;

    /// Prepares the record of the shape each task's tickets were built under, which
    /// [`MetaTicketApi::init`] reads.
    fn init_ticket_hash(&self) -> impl Future<Output = MetaResult<(), MSto::Error>> + Send;

    /// Prepares the record of the shape each dimension's resolutions were built under, which
    /// [`MetaResolutionApi::init`] reads.
    fn init_dimension_hash(&self) -> impl Future<Output = MetaResult<(), MSto::Error>> + Send;

    /// Prepares the backend's representation of [`TicketStatus`].
    fn init_ticket_status_type(&self) -> impl Future<Output = MetaResult<(), MSto::Error>> + Send;

    /// Prepares the per-task ticket counters that [`MetaTicketApi::get_status`] reads.
    fn init_ticket_summary(&self) -> impl Future<Output = MetaResult<(), MSto::Error>> + Send;

    // --- Footprint ---

    /// Initializes the footprint tables, returning their shape.
    ///
    /// Rebuilds them when the footprint's shape has changed, marking the run as aborted.
    /// Reports [`STALE`](TableShape::STALE) when that happened, [`CURRENT`](TableShape::CURRENT)
    /// otherwise.
    /// Backends that keep no record of the shape always report [`CURRENT`](TableShape::CURRENT).
    fn init_footprint(&self) -> impl Future<Output = MetaResult<TableShape, MSto::Error>> + Send;

    /// Discards the recorded run along with its executions.
    fn clear_footprint(&self) -> impl Future<Output = MetaResult<(), MSto::Error>> + Send;

    /// Reads the footprint of the run this store holds, or `None` if it holds none.
    fn get_footprint(
        &self,
    ) -> impl Future<Output = MetaResult<Option<RunFootprint>, MSto::Error>> + Send;

    /// Records `footprint` as the run this store holds, replacing any earlier one.
    ///
    /// A store holds one run at a time, so recording a different run displaces the previous one.
    fn upsert_run(
        &self,
        footprint: &RunFootprint,
    ) -> impl Future<Output = MetaResult<(), MSto::Error>> + Send;

    /// Records the start of an execution of the given run.
    ///
    /// Each `run` command starts one execution, so a run resumed several times accumulates one
    /// record per attempt.
    fn put_execution(
        &self,
        run_id: Uuid,
        execution_id: Uuid,
    ) -> impl Future<Output = MetaResult<(), MSto::Error>> + Send;

    /// Closes the given execution, recording when it ended and the state it ended in.
    fn update_execution_on_finish(
        &self,
        footprint: &RunFootprint,
        execution_id: Uuid,
    ) -> impl Future<Output = MetaResult<(), MSto::Error>> + Send;
}

/// Ticket operations for a task of arity `N`.
pub trait MetaTicketApi<const N: usize> {
    /// This builder's backend error type, matching its backend's [`MetaBackend::Error`].
    type Error: std::error::Error + Send + Sync + 'static;

    /// Initializes the ticket table for this task.
    ///
    /// Rebuilds the table when the task's shape has changed, discarding its tickets.
    /// Reports [`STALE`](TableShape::STALE) when that happened, [`CURRENT`](TableShape::CURRENT)
    /// otherwise.
    /// Backends that keep no record of the shape always report [`CURRENT`](TableShape::CURRENT).
    fn init(&self) -> impl Future<Output = MetaResult<TableShape, Self::Error>> + Send;

    /// Discards every ticket of this task, leaving what [`init`](Self::init) prepared in place.
    fn clear(&self) -> impl Future<Output = MetaResult<(), Self::Error>> + Send;

    /// Reads every ticket of this task that currently holds `status`.
    fn get_all(
        &self,
        status: TicketStatus,
    ) -> impl Future<Output = MetaResult<Vec<Ticket<N>>, Self::Error>> + Send;

    /// Stores `ticket`, leaving an existing ticket at the same coordinate untouched.
    fn put(&self, ticket: Ticket<N>) -> impl Future<Output = MetaResult<(), Self::Error>> + Send;

    /// Counts the finished `upstream_job` against every waiting ticket that depends on it, and
    /// returns the ones it made runnable.
    ///
    /// A ticket depends on the job when they agree on the dimensions both carry, ignoring the
    /// `aggregate_dims` this task aggregates over.
    fn raise_deps_done<const M: usize>(
        &self,
        upstream_meta: TaskMetadata<M>,
        upstream_job: Job<M>,
        aggregate_dims: &[&'static str],
    ) -> impl Future<Output = MetaResult<Vec<Ticket<N>>, Self::Error>> + Send;

    /// Raises by `ub - 1` the number of upstream jobs the tickets depending on `upstream_ticket`
    /// wait for, and returns the ones this made runnable.
    ///
    /// Resolving a spawned dimension to `ub` turns that one upstream ticket into `ub` jobs, so a
    /// ticket aggregating over the dimension gains the dependencies that were unknown when its
    /// quota was first set.
    fn raise_deps_quota<const M: usize>(
        &self,
        upstream_meta: TaskMetadata<M>,
        upstream_ticket: Ticket<M>,
        aggregate_dims: &[&'static str],
        ub: usize,
    ) -> impl Future<Output = MetaResult<Vec<Ticket<N>>, Self::Error>> + Send;

    /// Replaces each ticket left unresolved along the dimension at index `IDX` with one ticket per
    /// index that `res` resolves, and returns the tickets that were replaced.
    ///
    /// Errors when a ticket to be replaced already carries a coordinate at `IDX`, which would
    /// resolve the same dimension twice.
    fn explode<const M: usize, const IDX: usize>(
        &self,
        res_meta: DimensionMetadata<M>,
        res: Resolution<M>,
    ) -> impl Future<Output = MetaResult<Vec<Ticket<N>>, Self::Error>> + Send;

    /// Marks the ticket of `job` done.
    fn mark_done(&self, job: Job<N>) -> impl Future<Output = MetaResult<(), Self::Error>> + Send;

    /// Counts this task's tickets, as `(done, queued, waiting)`.
    fn get_status(&self) -> impl Future<Output = MetaResult<(i64, i64, i64), Self::Error>> + Send;
}

/// Resolution operations for a dimension of arity `N`.
pub trait MetaResolutionApi<const N: usize> {
    /// This builder's backend error type, matching its backend's [`MetaBackend::Error`].
    type Error: std::error::Error + Send + Sync + 'static;

    /// Initializes the resolution table for this dimension.
    ///
    /// Rebuilds the table when the dimension's shape has changed, discarding its resolutions.
    /// Reports [`STALE`](TableShape::STALE) when that happened, [`CURRENT`](TableShape::CURRENT)
    /// otherwise.
    /// Backends that keep no record of the shape always report [`CURRENT`](TableShape::CURRENT).
    fn init(&self) -> impl Future<Output = MetaResult<TableShape, Self::Error>> + Send;

    /// Discards every resolution of this dimension, leaving what [`init`](Self::init) prepared in
    /// place.
    fn clear(&self) -> impl Future<Output = MetaResult<(), Self::Error>> + Send;

    /// Reads the resolution stored at `coordinate`, or `None` if that coordinate holds none.
    fn get(
        &self,
        coordinate: [usize; N],
    ) -> impl Future<Output = MetaResult<Option<Resolution<N>>, Self::Error>> + Send;

    /// Stores `resolution`, leaving an existing resolution at the same coordinate untouched.
    fn put(
        &self,
        resolution: Resolution<N>,
    ) -> impl Future<Output = MetaResult<(), Self::Error>> + Send;
}
