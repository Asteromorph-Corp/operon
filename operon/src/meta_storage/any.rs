//! The runtime-selected metadata backend.
//!
//! [`AnyBackend`] is a [`MetaBackend`] whose concrete backend is chosen at runtime from
//! [`MetaBackendOptions`].
//! It is the default `MSto` for [`Operon`](crate::Operon), and is used to select between available
//! backends at runtime.

use uuid::Uuid;

use crate::meta_storage::mem::{
    MemClient, MemConn, MemMetaError, MemMetaStorage, MemResolutionQueryBuilder,
    MemTicketQueryBuilder, MemTx,
};
use crate::meta_storage::psql::{
    PsqlClient, PsqlConn, PsqlMetaError, PsqlMetaStorage, PsqlResolutionQueryBuilder,
    PsqlTicketQueryBuilder, PsqlTx,
};
use crate::meta_storage::{
    MetaBackend, MetaClientApi, MetaConnApi, MetaResolutionApi, MetaResult, MetaStorageError,
    MetaTicketApi, MetaTxApi,
};
use crate::schema::{
    DimensionMetadata, Job, JobMetadata, Resolution, RunFootprint, Ticket, TicketStatus,
};

/// Maps an operation over the backend variant and lifts the error.
macro_rules! map_lift_backend {
    ($self:expr, | $inner:ident | $call:expr) => {
        match $self {
            Self::Psql($inner) => $call.map_err(Into::into),
            Self::Mem($inner) => $call.map_err(Into::into),
        }
    };
    ($self:expr, | $inner:ident | $call:expr => $wrap:ident) => {
        match $self {
            Self::Psql($inner) => $call.map($wrap::Psql).map_err(Into::into),
            Self::Mem($inner) => $call.map($wrap::Mem).map_err(Into::into),
        }
    };
}

/// Maps an operation over the backend variant.
macro_rules! map_backend {
    ($self:expr, | $inner:ident | $call:expr => $wrap:ident) => {
        match $self {
            Self::Psql($inner) => $wrap::Psql($call),
            Self::Mem($inner) => $wrap::Mem($call),
        }
    };
}

/// A metadata backend selected at runtime from [`MetaBackendOptions`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum AnyBackend {
    Psql(PsqlMetaStorage),
    Mem(MemMetaStorage),
}

/// The error of a runtime-selected backend: the error of whichever backend was selected.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AnyBackendError {
    #[error(transparent)]
    Psql(PsqlMetaError),
    #[error(transparent)]
    Mem(MemMetaError),
}

/// Lifts a Postgres backend error into the runtime-selected backend's error.
impl From<MetaStorageError<PsqlMetaError>> for MetaStorageError<AnyBackendError> {
    fn from(err: MetaStorageError<PsqlMetaError>) -> Self {
        err.map_backend(AnyBackendError::Psql)
    }
}

/// Lifts an in-memory backend error into the runtime-selected backend's error.
impl From<MetaStorageError<MemMetaError>> for MetaStorageError<AnyBackendError> {
    fn from(err: MetaStorageError<MemMetaError>) -> Self {
        err.map_backend(AnyBackendError::Mem)
    }
}

impl MetaBackend for AnyBackend {
    type Error = AnyBackendError;
    type Conn<'a> = AnyConn<'a>;
    type Tx<'a> = AnyTx<'a>;
    type Client<'a> = AnyClient<'a>;
    type Ticket<'a, const N: usize> = AnyTicket<'a, N>;
    type Resolution<'a, const N: usize> = AnyResolution<'a, N>;

    async fn worker_conn(&self) -> MetaResult<AnyConn<'_>, AnyBackendError> {
        map_lift_backend!(self, |backend| backend.worker_conn().await => AnyConn)
    }

    async fn scheduler_conn(&self) -> MetaResult<AnyConn<'static>, AnyBackendError> {
        map_lift_backend!(self, |backend| backend.scheduler_conn().await => AnyConn)
    }

    async fn ensure_lock(&self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |backend| backend.ensure_lock().await)
    }

    async fn check_lock(&self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |backend| backend.check_lock().await)
    }
}

/// A checked-out connection over a runtime-selected backend.
#[allow(clippy::large_enum_variant)]
pub enum AnyConn<'a> {
    Psql(PsqlConn<'a>),
    Mem(MemConn),
}

impl MetaConnApi<AnyBackend> for AnyConn<'_> {
    async fn transaction(&mut self) -> MetaResult<AnyTx<'_>, AnyBackendError> {
        map_lift_backend!(self, |conn| conn.transaction().await => AnyTx)
    }

    fn as_client(&self) -> AnyClient<'_> {
        map_backend!(self, |conn| conn.as_client() => AnyClient)
    }
}

/// A transaction over a runtime-selected backend.
pub enum AnyTx<'a> {
    Psql(PsqlTx<'a>),
    Mem(MemTx<'a>),
}

impl MetaTxApi<AnyBackend> for AnyTx<'_> {
    async fn commit(self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |tx| tx.commit().await)
    }

    async fn rollback(self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |tx| tx.rollback().await)
    }

    fn as_client(&self) -> AnyClient<'_> {
        map_backend!(self, |tx| tx.as_client() => AnyClient)
    }
}

/// A query handle over a runtime-selected backend.
#[derive(Clone, Copy)]
pub enum AnyClient<'a> {
    Psql(PsqlClient<'a>),
    Mem(MemClient<'a>),
}

impl MetaClientApi<AnyBackend> for AnyClient<'_> {
    fn ticket<const N: usize>(&self, job_meta: JobMetadata<N>) -> AnyTicket<'_, N> {
        map_backend!(self, |client| client.ticket(job_meta) => AnyTicket)
    }

    fn resolution<const N: usize>(&self, dim_meta: DimensionMetadata<N>) -> AnyResolution<'_, N> {
        map_backend!(self, |client| client.resolution(dim_meta) => AnyResolution)
    }

    async fn init_schema(&self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |client| client.init_schema().await)
    }

    async fn init_ticket_hash(&self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |client| client.init_ticket_hash().await)
    }

    async fn init_dimension_hash(&self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |client| client.init_dimension_hash().await)
    }

    async fn init_ticket_status_type(&self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |client| client.init_ticket_status_type().await)
    }

    async fn init_ticket_summary(&self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |client| client.init_ticket_summary().await)
    }

    async fn init_footprint(&self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |client| client.init_footprint().await)
    }

    async fn clear_footprint(&self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |client| client.clear_footprint().await)
    }

    async fn get_footprint(&self) -> MetaResult<Option<RunFootprint>, AnyBackendError> {
        map_lift_backend!(self, |client| client.get_footprint().await)
    }

    async fn upsert_run(&self, footprint: &RunFootprint) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |client| client.upsert_run(footprint).await)
    }

    async fn put_execution(
        &self,
        run_id: Uuid,
        execution_id: Uuid,
    ) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |client| {
            client.put_execution(run_id, execution_id).await
        })
    }

    async fn update_execution_on_finish(
        &self,
        footprint: &RunFootprint,
        execution_id: Uuid,
    ) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |client| {
            client
                .update_execution_on_finish(footprint, execution_id)
                .await
        })
    }
}

/// A ticket query builder over a runtime-selected backend.
pub enum AnyTicket<'a, const N: usize> {
    Psql(PsqlTicketQueryBuilder<'a, N>),
    Mem(MemTicketQueryBuilder<'a, N>),
}

impl<const N: usize> MetaTicketApi<N> for AnyTicket<'_, N> {
    type Error = AnyBackendError;

    async fn init(&self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |ticket| ticket.init().await)
    }

    async fn clear(&self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |ticket| ticket.clear().await)
    }

    async fn get_all(&self, status: TicketStatus) -> MetaResult<Vec<Ticket<N>>, AnyBackendError> {
        map_lift_backend!(self, |ticket| ticket.get_all(status).await)
    }

    async fn put(&self, ticket: Ticket<N>) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |builder| builder.put(ticket).await)
    }

    async fn raise_deps_done<const M: usize>(
        &self,
        upstream_meta: JobMetadata<M>,
        upstream_job: Job<M>,
        aggregate_dims: &[&'static str],
    ) -> MetaResult<Vec<Ticket<N>>, AnyBackendError> {
        map_lift_backend!(self, |ticket| {
            ticket
                .raise_deps_done(upstream_meta, upstream_job, aggregate_dims)
                .await
        })
    }

    async fn raise_deps_quota<const M: usize>(
        &self,
        upstream_meta: JobMetadata<M>,
        upstream_ticket: Ticket<M>,
        aggregate_dims: &[&'static str],
        ub: usize,
    ) -> MetaResult<Vec<Ticket<N>>, AnyBackendError> {
        map_lift_backend!(self, |ticket| {
            ticket
                .raise_deps_quota(upstream_meta, upstream_ticket, aggregate_dims, ub)
                .await
        })
    }

    async fn explode<const M: usize, const IDX: usize>(
        &self,
        res_meta: DimensionMetadata<M>,
        res: Resolution<M>,
    ) -> MetaResult<Vec<Ticket<N>>, AnyBackendError> {
        map_lift_backend!(self, |ticket| ticket.explode::<M, IDX>(res_meta, res).await)
    }

    async fn mark_done(&self, job: Job<N>) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |ticket| ticket.mark_done(job).await)
    }

    async fn get_status(&self) -> MetaResult<(i64, i64, i64), AnyBackendError> {
        map_lift_backend!(self, |ticket| ticket.get_status().await)
    }
}

/// A resolution query builder over a runtime-selected backend.
pub enum AnyResolution<'a, const N: usize> {
    Psql(PsqlResolutionQueryBuilder<'a, N>),
    Mem(MemResolutionQueryBuilder<'a, N>),
}

impl<const N: usize> MetaResolutionApi<N> for AnyResolution<'_, N> {
    type Error = AnyBackendError;

    async fn init(&self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |resolution| resolution.init().await)
    }

    async fn clear(&self) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |resolution| resolution.clear().await)
    }

    async fn get(
        &self,
        coordinate: [usize; N],
    ) -> MetaResult<Option<Resolution<N>>, AnyBackendError> {
        map_lift_backend!(self, |resolution| resolution.get(coordinate).await)
    }

    async fn put(&self, resolution: Resolution<N>) -> MetaResult<(), AnyBackendError> {
        map_lift_backend!(self, |builder| builder.put(resolution).await)
    }
}
