use std::borrow::Cow;
use std::fmt::Display;

use bytes::Buf;
use deadpool_postgres::Transaction;
use tokio_postgres::{CopyInSink, ToStatement};

use crate::storage::psql::PsqlStorageResult;
use crate::utils::SchemaPrefix;

macro_rules! impl_storage_client {
    (
        $(#[$attr:meta])?
        $method:ident
        $(< $($generics:tt),* >)?
        ( $($arg:ident : $arg_ty:ty),* ) -> $ret:ty
        $(where $($where_clause:tt)+)?
    ) => {
        $(#[$attr])?
        pub async fn $method
        $(< $($generics),* >)?
        (&self, $($arg: $arg_ty),*)  -> PsqlStorageResult<$ret>
        $(where $($where_clause)+)?
        {
            self.client.$method($($arg),*).await.map_err(Into::into)
        }
    };
}

type ToSql = dyn tokio_postgres::types::ToSql + Sync;

/// A schema-aware wrapper for a Postgres client.
#[derive(Debug)]
pub struct StorageClient<'a> {
    client: deadpool_postgres::Object,
    schema: Option<Cow<'a, str>>,
}

impl<'a> StorageClient<'a> {
    /// Creates a new `StorageClient` with the given Postgres client and optional schema.
    pub fn new(client: deadpool_postgres::Object, schema: Option<impl Into<Cow<'a, str>>>) -> Self {
        let schema = schema.map(Into::into);
        StorageClient { client, schema }
    }

    impl_storage_client! {
        /// Executes a batch of SQL statements.
        batch_execute(query: &str) -> ()
    }
    impl_storage_client! {
        /// Executes a single SQL statement.
        execute(query: &str, params: &[&ToSql]) -> u64
    }
    impl_storage_client! {
        /// Executes a SQL query and returns the resulting rows.
        query(query: &str, params: &[&ToSql]) -> Vec<tokio_postgres::Row>
    }
    impl_storage_client! {
        /// Executes a SQL query and returns the first resulting row, if it exists.
        query_opt(query: &str, params: &[&ToSql]) -> Option<tokio_postgres::Row>
    }
    impl_storage_client! {
        /// Initiates a COPY IN operation.
        copy_in<T, U>(query: &T) -> CopyInSink<U>
        where T: ?Sized + ToStatement + Send + Sync,
              U: Buf + 'static + Send + Sync
    }

    /// Executes a batch of SQL statements wrapped in a `Display` type.
    pub async fn batch_execute_stmt(&self, stmt: &impl Display) -> PsqlStorageResult<()> {
        self.batch_execute(&stmt.to_string()).await
    }

    /// Executes a single SQL statement wrapped in a `Display` type.
    pub async fn execute_stmt(
        &self,
        stmt: &impl Display,
        params: &[&ToSql],
    ) -> PsqlStorageResult<u64> {
        self.execute(&stmt.to_string(), params).await
    }

    /// Executes a SQL query and returns the resulting rows, with the query wrapped in a `Display`
    /// type.
    pub async fn query_stmt(
        &self,
        stmt: &impl Display,
        params: &[&ToSql],
    ) -> PsqlStorageResult<Vec<tokio_postgres::Row>> {
        self.query(&stmt.to_string(), params).await
    }

    /// Executes a SQL query and returns the first resulting row, if it exists, with the query
    /// wrapped in a `Display` type.
    pub async fn query_opt_stmt(
        &self,
        stmt: &impl Display,
        params: &[&ToSql],
    ) -> PsqlStorageResult<Option<tokio_postgres::Row>> {
        self.query_opt(&stmt.to_string(), params).await
    }

    /// Returns the schema name used by this storage client, if any.
    pub fn schema(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    /// Returns the printable schema prefix for this storage client, if any.
    pub fn schema_prefix(&self) -> SchemaPrefix<'_> {
        SchemaPrefix(self.schema())
    }

    /// Begins a new transaction on the underlying Postgres client.
    pub async fn transaction(&mut self) -> PsqlStorageResult<Transaction<'_>> {
        let tx = self.client.transaction().await?;
        Ok(tx)
    }
}
