use std::borrow::Cow;
use std::fmt::Display;

use bytes::Buf;
use tokio_postgres::{CopyInSink, ToStatement};
use uuid::Uuid;

use crate::meta_storage::psql::error::PsqlResult;
use crate::meta_storage::psql::{
    PsqlMetaError, PsqlMetaStorage, PsqlResolutionQueryBuilder, PsqlTicketQueryBuilder,
};
use crate::meta_storage::{MetaClientApi, MetaConnApi, MetaTxApi};
use crate::schema::{DimensionMetadata, JobMetadata, RunFootprint, TableShape};
use crate::utils::SchemaPrefix;

macro_rules! impl_psql_client {
    (
        $method:ident
        $(< $($generics:tt),* >)?
        ( $($arg:ident : $arg_ty:ty),* ) -> $ret:ty
        $(where $($where_clause:tt)+)?
    ) => {
        pub async fn $method
        $(< $($generics),* >)?
        (&self, $($arg: $arg_ty),*)  -> PsqlResult<$ret>
        $(where $($where_clause)+)?
        {
            match self {
                PsqlClient::Object(PsqlConn { client, .. }) => {
                    client.$method($($arg),*).await.map_err(|e| PsqlMetaError::from(e).into())
                }
                PsqlClient::Transaction(PsqlTx { tx, .. }) => {
                    tx.$method($($arg),*).await.map_err(|e| PsqlMetaError::from(e).into())
                }
            }
        }
    };
}

type ToSql = dyn tokio_postgres::types::ToSql + Sync;

/// A pooled Postgres connection paired with the optional metadata schema it operates against.
#[derive(Debug)]
pub struct PsqlConn<'a> {
    client: deadpool_postgres::Object,
    schema: Option<Cow<'a, str>>,
}

impl<'a> PsqlConn<'a> {
    pub fn new(client: deadpool_postgres::Object, schema: Option<impl Into<Cow<'a, str>>>) -> Self {
        let schema = schema.map(Into::into);
        PsqlConn { client, schema }
    }
}

impl MetaConnApi<PsqlMetaStorage> for PsqlConn<'_> {
    async fn transaction(&mut self) -> PsqlResult<PsqlTx<'_>> {
        let tx = self
            .client
            .transaction()
            .await
            .map_err(PsqlMetaError::from)?;
        let schema = self.schema.as_deref().map(Cow::Borrowed);
        Ok(PsqlTx { tx, schema })
    }

    fn as_client(&self) -> PsqlClient<'_> {
        PsqlClient::Object(self)
    }
}

/// A Postgres transaction paired with the optional metadata schema it operates against.
#[derive(Debug)]
pub struct PsqlTx<'a> {
    tx: deadpool_postgres::Transaction<'a>,
    schema: Option<Cow<'a, str>>,
}

impl MetaTxApi<PsqlMetaStorage> for PsqlTx<'_> {
    async fn commit(self) -> PsqlResult<()> {
        self.tx
            .commit()
            .await
            .map_err(|e| PsqlMetaError::from(e).into())
    }

    async fn rollback(self) -> PsqlResult<()> {
        self.tx
            .rollback()
            .await
            .map_err(|e| PsqlMetaError::from(e).into())
    }

    fn as_client(&self) -> PsqlClient<'_> {
        PsqlClient::Transaction(self)
    }
}

/// A borrowed handle over either a [`PsqlConn`] or a [`PsqlTx`].
#[derive(Debug, Clone, Copy)]
pub enum PsqlClient<'a> {
    Object(&'a PsqlConn<'a>),
    Transaction(&'a PsqlTx<'a>),
}

impl PsqlClient<'_> {
    impl_psql_client!(batch_execute(query: &str) -> ());
    impl_psql_client!(execute(query: &str, params: &[&ToSql]) -> u64);
    impl_psql_client!(query(query: &str, params: &[&ToSql]) -> Vec<tokio_postgres::Row>);
    impl_psql_client!(query_opt(query: &str, params: &[&ToSql]) -> Option<tokio_postgres::Row>);
    impl_psql_client!(
        copy_in<T, U>(query: &T) -> CopyInSink<U>
        where T: ?Sized + ToStatement + Send + Sync,
              U: Buf + 'static + Send + Sync
    );

    pub async fn execute_stmt(&self, stmt: &impl Display, params: &[&ToSql]) -> PsqlResult<u64> {
        self.execute(&stmt.to_string(), params).await
    }

    pub async fn query_stmt(
        &self,
        stmt: &impl Display,
        params: &[&ToSql],
    ) -> PsqlResult<Vec<tokio_postgres::Row>> {
        self.query(&stmt.to_string(), params).await
    }

    pub async fn query_opt_stmt(
        &self,
        stmt: &impl Display,
        params: &[&ToSql],
    ) -> PsqlResult<Option<tokio_postgres::Row>> {
        self.query_opt(&stmt.to_string(), params).await
    }

    pub fn schema(&self) -> Option<&str> {
        match self {
            PsqlClient::Object(PsqlConn { schema, .. }) => schema.as_deref(),
            PsqlClient::Transaction(PsqlTx { schema, .. }) => schema.as_deref(),
        }
    }

    pub fn schema_prefix(&self) -> SchemaPrefix<'_> {
        SchemaPrefix(self.schema())
    }
}

impl MetaClientApi<PsqlMetaStorage> for PsqlClient<'_> {
    fn ticket<const N: usize>(&self, job_meta: JobMetadata<N>) -> PsqlTicketQueryBuilder<'_, N> {
        PsqlClient::ticket(self, job_meta)
    }

    fn resolution<const N: usize>(
        &self,
        dim_meta: DimensionMetadata<N>,
    ) -> PsqlResolutionQueryBuilder<'_, N> {
        PsqlClient::resolution(self, dim_meta)
    }

    async fn init_schema(&self) -> PsqlResult<()> {
        PsqlClient::init_schema(self).await
    }

    async fn init_ticket_hash(&self) -> PsqlResult<()> {
        PsqlClient::init_ticket_hash(self).await
    }

    async fn init_dimension_hash(&self) -> PsqlResult<()> {
        PsqlClient::init_dimension_hash(self).await
    }

    async fn init_ticket_status_type(&self) -> PsqlResult<()> {
        PsqlClient::init_ticket_status_type(self).await
    }

    async fn init_ticket_summary(&self) -> PsqlResult<()> {
        PsqlClient::init_ticket_summary(self).await
    }

    async fn init_footprint(&self) -> PsqlResult<TableShape> {
        PsqlClient::init_footprint(self).await
    }

    async fn clear_footprint(&self) -> PsqlResult<()> {
        PsqlClient::clear_footprint(self).await
    }

    async fn get_footprint(&self) -> PsqlResult<Option<RunFootprint>> {
        PsqlClient::get_footprint(self).await
    }

    async fn upsert_run(&self, footprint: &RunFootprint) -> PsqlResult<()> {
        PsqlClient::upsert_run(self, footprint).await
    }

    async fn put_execution(&self, run_id: Uuid, execution_id: Uuid) -> PsqlResult<()> {
        PsqlClient::put_execution(self, run_id, execution_id).await
    }

    async fn update_execution_on_finish(
        &self,
        footprint: &RunFootprint,
        execution_id: Uuid,
    ) -> PsqlResult<()> {
        PsqlClient::update_execution_on_finish(self, footprint, execution_id).await
    }
}

impl<'a> From<&'a PsqlConn<'a>> for PsqlClient<'a> {
    fn from(client: &'a PsqlConn<'a>) -> Self {
        PsqlClient::Object(client)
    }
}

impl<'a> From<&'a PsqlTx<'a>> for PsqlClient<'a> {
    fn from(tx: &'a PsqlTx<'a>) -> Self {
        PsqlClient::Transaction(tx)
    }
}
