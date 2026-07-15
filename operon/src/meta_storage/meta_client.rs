use uuid::Uuid;

use crate::meta_storage::MetaStorageError;
use crate::meta_storage::psql::{PsqlClient, PsqlConn, PsqlTx};
use crate::schema::{
    DimensionMetadata, Job, JobMetadata, Resolution, RunFootprint, Ticket, TicketStatus,
};

/// A backend-agnostic checked-out metadata connection.
///
/// Wraps a concrete backend connection; obtained from
/// [`MetaStorage::worker_conn`](crate::meta_storage::MetaStorage::worker_conn) or
/// [`scheduler_conn`](crate::meta_storage::MetaStorage::scheduler_conn).
#[derive(Debug)]
#[non_exhaustive]
pub enum MetaConn<'a> {
    Psql(PsqlConn<'a>),
}

impl<'a> MetaConn<'a> {
    /// Begins a transaction over this connection.
    pub async fn transaction(&'a mut self) -> Result<MetaTx<'a>, MetaStorageError> {
        match self {
            MetaConn::Psql(conn) => Ok(MetaTx::Psql(conn.transaction().await?)),
        }
    }

    /// Borrows this connection as a [`MetaClient`] for issuing queries.
    pub fn as_client(&self) -> MetaClient<'_> {
        match self {
            MetaConn::Psql(conn) => MetaClient::Psql(conn.as_client()),
        }
    }
}

/// A backend-agnostic metadata transaction.
#[derive(Debug)]
#[non_exhaustive]
pub enum MetaTx<'a> {
    Psql(PsqlTx<'a>),
}

impl<'a> MetaTx<'a> {
    pub async fn commit(self) -> Result<(), MetaStorageError> {
        match self {
            MetaTx::Psql(tx) => tx.commit().await,
        }
    }

    pub async fn rollback(self) -> Result<(), MetaStorageError> {
        match self {
            MetaTx::Psql(tx) => tx.rollback().await,
        }
    }

    /// Borrows this transaction as a [`MetaClient`] for issuing queries.
    pub fn as_client(&self) -> MetaClient<'_> {
        match self {
            MetaTx::Psql(tx) => MetaClient::Psql(tx.as_client()),
        }
    }
}

/// A backend-agnostic handle for issuing metadata queries.
///
/// All metadata operations are exposed here in terms of the domain types (`Ticket`, `Resolution`,
/// `RunFootprint`, …); translation to the concrete backend happens per-operation. Adding a backend
/// is a matter of adding a variant and an arm to each method.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum MetaClient<'a> {
    Psql(PsqlClient<'a>),
}

impl<'a> MetaClient<'a> {
    /// Builds ticket queries for the given job.
    pub fn ticket<const N: usize>(&'a self, job_meta: JobMetadata<N>) -> TicketQueryBuilder<'a, N> {
        TicketQueryBuilder {
            client: self,
            job_meta,
        }
    }

    /// Builds resolution queries for the given dimension.
    pub fn resolution<const N: usize>(
        &'a self,
        dim_meta: DimensionMetadata<N>,
    ) -> ResolutionQueryBuilder<'a, N> {
        ResolutionQueryBuilder {
            client: self,
            dim_meta,
        }
    }

    // --- Schema initialization ---

    pub async fn init_schema(&self) -> Result<(), MetaStorageError> {
        match self {
            MetaClient::Psql(client) => client.init_schema().await,
        }
    }

    pub async fn init_ticket_hash(&self) -> Result<(), MetaStorageError> {
        match self {
            MetaClient::Psql(client) => client.init_ticket_hash().await,
        }
    }

    pub async fn init_dimension_hash(&self) -> Result<(), MetaStorageError> {
        match self {
            MetaClient::Psql(client) => client.init_dimension_hash().await,
        }
    }

    pub async fn init_ticket_status_type(&self) -> Result<(), MetaStorageError> {
        match self {
            MetaClient::Psql(client) => client.init_ticket_status_type().await,
        }
    }

    pub async fn init_ticket_summary(&self) -> Result<(), MetaStorageError> {
        match self {
            MetaClient::Psql(client) => client.init_ticket_summary().await,
        }
    }

    // --- Footprint ---

    pub async fn init_footprint(&self) -> Result<(), MetaStorageError> {
        match self {
            MetaClient::Psql(client) => client.init_footprint().await,
        }
    }

    pub async fn clear_footprint(&self) -> Result<(), MetaStorageError> {
        match self {
            MetaClient::Psql(client) => client.clear_footprint().await,
        }
    }

    pub async fn get_footprint(&self) -> Result<Option<RunFootprint>, MetaStorageError> {
        match self {
            MetaClient::Psql(client) => client.get_footprint().await,
        }
    }

    pub async fn upsert_run(&self, footprint: &RunFootprint) -> Result<(), MetaStorageError> {
        match self {
            MetaClient::Psql(client) => client.upsert_run(footprint).await,
        }
    }

    pub async fn put_execution(
        &self,
        run_id: Uuid,
        execution_id: Uuid,
    ) -> Result<(), MetaStorageError> {
        match self {
            MetaClient::Psql(client) => client.put_execution(run_id, execution_id).await,
        }
    }

    pub async fn update_execution_on_finish(
        &self,
        footprint: &RunFootprint,
        execution_id: Uuid,
    ) -> Result<(), MetaStorageError> {
        match self {
            MetaClient::Psql(client) => {
                client
                    .update_execution_on_finish(footprint, execution_id)
                    .await
            }
        }
    }
}

/// Backend-agnostic builder for ticket queries, delegating to the concrete backend per-operation.
pub struct TicketQueryBuilder<'a, const N: usize> {
    client: &'a MetaClient<'a>,
    job_meta: JobMetadata<N>,
}

impl<const N: usize> TicketQueryBuilder<'_, N> {
    pub async fn init(&self) -> Result<(), MetaStorageError> {
        match self.client {
            MetaClient::Psql(client) => client.ticket(self.job_meta).init().await,
        }
    }

    pub async fn clear(&self) -> Result<(), MetaStorageError> {
        match self.client {
            MetaClient::Psql(client) => client.ticket(self.job_meta).clear().await,
        }
    }

    pub async fn get_all(&self, status: TicketStatus) -> Result<Vec<Ticket<N>>, MetaStorageError> {
        match self.client {
            MetaClient::Psql(client) => client.ticket(self.job_meta).get_all(status).await,
        }
    }

    pub async fn put(&self, ticket: Ticket<N>) -> Result<(), MetaStorageError> {
        match self.client {
            MetaClient::Psql(client) => client.ticket(self.job_meta).put(ticket).await,
        }
    }

    pub async fn raise_deps_done<const M: usize>(
        &self,
        upstream_meta: JobMetadata<M>,
        upstream_job: Job<M>,
        aggregate_dims: &[&'static str],
    ) -> Result<Vec<Ticket<N>>, MetaStorageError> {
        match self.client {
            MetaClient::Psql(client) => {
                client
                    .ticket(self.job_meta)
                    .raise_deps_done(upstream_meta, upstream_job, aggregate_dims)
                    .await
            }
        }
    }

    pub async fn raise_deps_quota<const M: usize>(
        &self,
        upstream_meta: JobMetadata<M>,
        upstream_ticket: Ticket<M>,
        aggregate_dims: &[&'static str],
        ub: usize,
    ) -> Result<Vec<Ticket<N>>, MetaStorageError> {
        match self.client {
            MetaClient::Psql(client) => {
                client
                    .ticket(self.job_meta)
                    .raise_deps_quota(upstream_meta, upstream_ticket, aggregate_dims, ub)
                    .await
            }
        }
    }

    pub async fn explode<const M: usize, const IDX: usize>(
        &self,
        res_meta: DimensionMetadata<M>,
        res: Resolution<M>,
    ) -> Result<Vec<Ticket<N>>, MetaStorageError> {
        match self.client {
            MetaClient::Psql(client) => {
                client
                    .ticket(self.job_meta)
                    .explode::<M, IDX>(res_meta, res)
                    .await
            }
        }
    }

    pub async fn mark_done(&self, job: Job<N>) -> Result<(), MetaStorageError> {
        match self.client {
            MetaClient::Psql(client) => client.ticket(self.job_meta).mark_done(job).await,
        }
    }

    pub async fn get_status(&self) -> Result<(i64, i64, i64), MetaStorageError> {
        match self.client {
            MetaClient::Psql(client) => client.ticket(self.job_meta).get_status().await,
        }
    }
}

/// Backend-agnostic builder for resolution queries, delegating to the concrete backend
/// per-operation.
pub struct ResolutionQueryBuilder<'a, const N: usize> {
    client: &'a MetaClient<'a>,
    dim_meta: DimensionMetadata<N>,
}

impl<const N: usize> ResolutionQueryBuilder<'_, N> {
    pub async fn init(&self) -> Result<(), MetaStorageError> {
        match self.client {
            MetaClient::Psql(client) => client.resolution(self.dim_meta).init().await,
        }
    }

    pub async fn clear(&self) -> Result<(), MetaStorageError> {
        match self.client {
            MetaClient::Psql(client) => client.resolution(self.dim_meta).clear().await,
        }
    }

    pub async fn get(
        &self,
        coordinate: [usize; N],
    ) -> Result<Option<Resolution<N>>, MetaStorageError> {
        match self.client {
            MetaClient::Psql(client) => client.resolution(self.dim_meta).get(coordinate).await,
        }
    }

    pub async fn put(&self, resolution: Resolution<N>) -> Result<(), MetaStorageError> {
        match self.client {
            MetaClient::Psql(client) => client.resolution(self.dim_meta).put(resolution).await,
        }
    }
}
