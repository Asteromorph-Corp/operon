//! The runtime-selected metadata backend.
//!
//! [`AnyBackend`] is a [`MetaBackend`] whose concrete backend is chosen at runtime from
//! [`MetaBackendOptions`]. It is the default `MSto` for [`Operon`](crate::Operon): a program that
//! doesn't pin a backend at compile time runs against `AnyBackend`, which match-dispatches every
//! operation to the selected backend through hand-written wrapper enums.

use uuid::Uuid;

use crate::meta_storage::psql::{
    PsqlClient, PsqlConn, PsqlMetaError, PsqlMetaStorage, PsqlResolutionQueryBuilder,
    PsqlTicketQueryBuilder, PsqlTx,
};
use crate::meta_storage::{
    MetaBackend, MetaBackendOptions, MetaClientApi, MetaConnApi, MetaResolutionApi, MetaResult,
    MetaStorageError, MetaTicketApi, MetaTxApi,
};
use crate::schema::{
    DimensionMetadata, Job, JobMetadata, Resolution, RunFootprint, Ticket, TicketStatus,
};

/// A metadata backend selected at runtime from [`MetaBackendOptions`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum AnyBackend {
    Psql(PsqlMetaStorage),
}

/// The error of a runtime-selected backend.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AnyBackendError {
    #[error(transparent)]
    Psql(PsqlMetaError),
}

/// Lifts a Postgres backend error into the runtime-selected backend's error.
fn lift_psql(err: MetaStorageError<PsqlMetaError>) -> MetaStorageError<AnyBackendError> {
    err.map_backend(AnyBackendError::Psql)
}

impl MetaBackend for AnyBackend {
    type Options = MetaBackendOptions;
    type Error = AnyBackendError;
    type Conn<'a> = AnyConn<'a>;
    type Tx<'a> = AnyTx<'a>;
    type Client<'a> = AnyClient<'a>;
    type Ticket<'a, const N: usize> = AnyTicket<'a, N>;
    type Resolution<'a, const N: usize> = AnyResolution<'a, N>;

    fn new(options: MetaBackendOptions) -> MetaResult<Self, AnyBackendError> {
        match options {
            MetaBackendOptions::Psql(options) => Ok(AnyBackend::Psql(
                PsqlMetaStorage::new(options).map_err(lift_psql)?,
            )),
        }
    }

    async fn worker_conn(&self) -> MetaResult<AnyConn<'_>, AnyBackendError> {
        match self {
            AnyBackend::Psql(backend) => backend
                .worker_conn()
                .await
                .map(AnyConn::Psql)
                .map_err(lift_psql),
        }
    }

    async fn scheduler_conn(&self) -> MetaResult<AnyConn<'static>, AnyBackendError> {
        match self {
            AnyBackend::Psql(backend) => backend
                .scheduler_conn()
                .await
                .map(AnyConn::Psql)
                .map_err(lift_psql),
        }
    }

    async fn ensure_lock(&self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyBackend::Psql(backend) => backend.ensure_lock().await.map_err(lift_psql),
        }
    }

    async fn check_lock(&self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyBackend::Psql(backend) => backend.check_lock().await.map_err(lift_psql),
        }
    }
}

/// A checked-out connection over a runtime-selected backend.
pub enum AnyConn<'a> {
    Psql(PsqlConn<'a>),
}

impl MetaConnApi<AnyBackend> for AnyConn<'_> {
    async fn transaction(&mut self) -> MetaResult<AnyTx<'_>, AnyBackendError> {
        match self {
            AnyConn::Psql(conn) => conn.transaction().await.map(AnyTx::Psql).map_err(lift_psql),
        }
    }

    fn as_client(&self) -> AnyClient<'_> {
        match self {
            AnyConn::Psql(conn) => AnyClient::Psql(conn.as_client()),
        }
    }
}

/// A transaction over a runtime-selected backend.
pub enum AnyTx<'a> {
    Psql(PsqlTx<'a>),
}

impl MetaTxApi<AnyBackend> for AnyTx<'_> {
    async fn commit(self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyTx::Psql(tx) => tx.commit().await.map_err(lift_psql),
        }
    }

    async fn rollback(self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyTx::Psql(tx) => tx.rollback().await.map_err(lift_psql),
        }
    }

    fn as_client(&self) -> AnyClient<'_> {
        match self {
            AnyTx::Psql(tx) => AnyClient::Psql(tx.as_client()),
        }
    }
}

/// A query handle over a runtime-selected backend.
#[derive(Clone, Copy)]
pub enum AnyClient<'a> {
    Psql(PsqlClient<'a>),
}

impl MetaClientApi<AnyBackend> for AnyClient<'_> {
    fn ticket<const N: usize>(&self, job_meta: JobMetadata<N>) -> AnyTicket<'_, N> {
        match self {
            AnyClient::Psql(client) => AnyTicket::Psql(client.ticket(job_meta)),
        }
    }

    fn resolution<const N: usize>(&self, dim_meta: DimensionMetadata<N>) -> AnyResolution<'_, N> {
        match self {
            AnyClient::Psql(client) => AnyResolution::Psql(client.resolution(dim_meta)),
        }
    }

    async fn init_schema(&self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyClient::Psql(client) => client.init_schema().await.map_err(lift_psql),
        }
    }

    async fn init_ticket_hash(&self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyClient::Psql(client) => client.init_ticket_hash().await.map_err(lift_psql),
        }
    }

    async fn init_dimension_hash(&self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyClient::Psql(client) => client.init_dimension_hash().await.map_err(lift_psql),
        }
    }

    async fn init_ticket_status_type(&self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyClient::Psql(client) => client.init_ticket_status_type().await.map_err(lift_psql),
        }
    }

    async fn init_ticket_summary(&self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyClient::Psql(client) => client.init_ticket_summary().await.map_err(lift_psql),
        }
    }

    async fn init_footprint(&self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyClient::Psql(client) => client.init_footprint().await.map_err(lift_psql),
        }
    }

    async fn clear_footprint(&self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyClient::Psql(client) => client.clear_footprint().await.map_err(lift_psql),
        }
    }

    async fn get_footprint(&self) -> MetaResult<Option<RunFootprint>, AnyBackendError> {
        match self {
            AnyClient::Psql(client) => client.get_footprint().await.map_err(lift_psql),
        }
    }

    async fn upsert_run(&self, footprint: &RunFootprint) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyClient::Psql(client) => client.upsert_run(footprint).await.map_err(lift_psql),
        }
    }

    async fn put_execution(
        &self,
        run_id: Uuid,
        execution_id: Uuid,
    ) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyClient::Psql(client) => client
                .put_execution(run_id, execution_id)
                .await
                .map_err(lift_psql),
        }
    }

    async fn update_execution_on_finish(
        &self,
        footprint: &RunFootprint,
        execution_id: Uuid,
    ) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyClient::Psql(client) => client
                .update_execution_on_finish(footprint, execution_id)
                .await
                .map_err(lift_psql),
        }
    }
}

/// A ticket query builder over a runtime-selected backend.
pub enum AnyTicket<'a, const N: usize> {
    Psql(PsqlTicketQueryBuilder<'a, N>),
}

impl<const N: usize> MetaTicketApi<N> for AnyTicket<'_, N> {
    type Error = AnyBackendError;

    async fn init(&self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyTicket::Psql(ticket) => ticket.init().await.map_err(lift_psql),
        }
    }

    async fn clear(&self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyTicket::Psql(ticket) => ticket.clear().await.map_err(lift_psql),
        }
    }

    async fn get_all(&self, status: TicketStatus) -> MetaResult<Vec<Ticket<N>>, AnyBackendError> {
        match self {
            AnyTicket::Psql(ticket) => ticket.get_all(status).await.map_err(lift_psql),
        }
    }

    async fn put(&self, ticket: Ticket<N>) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyTicket::Psql(builder) => builder.put(ticket).await.map_err(lift_psql),
        }
    }

    async fn raise_deps_done<const M: usize>(
        &self,
        upstream_meta: JobMetadata<M>,
        upstream_job: Job<M>,
        aggregate_dims: &[&'static str],
    ) -> MetaResult<Vec<Ticket<N>>, AnyBackendError> {
        match self {
            AnyTicket::Psql(ticket) => ticket
                .raise_deps_done(upstream_meta, upstream_job, aggregate_dims)
                .await
                .map_err(lift_psql),
        }
    }

    async fn raise_deps_quota<const M: usize>(
        &self,
        upstream_meta: JobMetadata<M>,
        upstream_ticket: Ticket<M>,
        aggregate_dims: &[&'static str],
        ub: usize,
    ) -> MetaResult<Vec<Ticket<N>>, AnyBackendError> {
        match self {
            AnyTicket::Psql(ticket) => ticket
                .raise_deps_quota(upstream_meta, upstream_ticket, aggregate_dims, ub)
                .await
                .map_err(lift_psql),
        }
    }

    async fn explode<const M: usize, const IDX: usize>(
        &self,
        res_meta: DimensionMetadata<M>,
        res: Resolution<M>,
    ) -> MetaResult<Vec<Ticket<N>>, AnyBackendError> {
        match self {
            AnyTicket::Psql(ticket) => ticket
                .explode::<M, IDX>(res_meta, res)
                .await
                .map_err(lift_psql),
        }
    }

    async fn mark_done(&self, job: Job<N>) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyTicket::Psql(ticket) => ticket.mark_done(job).await.map_err(lift_psql),
        }
    }

    async fn get_status(&self) -> MetaResult<(i64, i64, i64), AnyBackendError> {
        match self {
            AnyTicket::Psql(ticket) => ticket.get_status().await.map_err(lift_psql),
        }
    }
}

/// A resolution query builder over a runtime-selected backend.
pub enum AnyResolution<'a, const N: usize> {
    Psql(PsqlResolutionQueryBuilder<'a, N>),
}

impl<const N: usize> MetaResolutionApi<N> for AnyResolution<'_, N> {
    type Error = AnyBackendError;

    async fn init(&self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyResolution::Psql(resolution) => resolution.init().await.map_err(lift_psql),
        }
    }

    async fn clear(&self) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyResolution::Psql(resolution) => resolution.clear().await.map_err(lift_psql),
        }
    }

    async fn get(
        &self,
        coordinate: [usize; N],
    ) -> MetaResult<Option<Resolution<N>>, AnyBackendError> {
        match self {
            AnyResolution::Psql(resolution) => resolution.get(coordinate).await.map_err(lift_psql),
        }
    }

    async fn put(&self, resolution: Resolution<N>) -> MetaResult<(), AnyBackendError> {
        match self {
            AnyResolution::Psql(builder) => builder.put(resolution).await.map_err(lift_psql),
        }
    }
}
