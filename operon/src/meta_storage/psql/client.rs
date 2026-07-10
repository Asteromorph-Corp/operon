use std::borrow::Cow;
use std::fmt::Display;

use bytes::Buf;
use tokio_postgres::{CopyInSink, ToStatement};

use crate::meta_storage::MetaStorageError;
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
        (&self, $($arg: $arg_ty),*)  -> Result<$ret, MetaStorageError>
        $(where $($where_clause)+)?
        {
            match self {
                PsqlClient::Object(PsqlConn { client, .. }) => {
                    client.$method($($arg),*).await.map_err(Into::into)
                }
                PsqlClient::Transaction(PsqlTx { tx, .. }) => {
                    tx.$method($($arg),*).await.map_err(Into::into)
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

    pub async fn transaction(&'a mut self) -> Result<PsqlTx<'a>, MetaStorageError> {
        let tx = self.client.transaction().await?;
        let schema = self.schema.as_deref().map(Cow::Borrowed);
        Ok(PsqlTx { tx, schema })
    }

    pub fn as_client(&self) -> PsqlClient<'_> {
        PsqlClient::Object(self)
    }
}

/// A Postgres transaction paired with the optional metadata schema it operates against.
#[derive(Debug)]
pub struct PsqlTx<'a> {
    tx: deadpool_postgres::Transaction<'a>,
    schema: Option<Cow<'a, str>>,
}

impl<'a> PsqlTx<'a> {
    pub async fn commit(self) -> Result<(), MetaStorageError> {
        self.tx.commit().await.map_err(Into::into)
    }

    pub async fn rollback(self) -> Result<(), MetaStorageError> {
        self.tx.rollback().await.map_err(Into::into)
    }

    pub fn as_client(&self) -> PsqlClient<'_> {
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

    pub async fn batch_execute_stmt(&self, stmt: &impl Display) -> Result<(), MetaStorageError> {
        self.batch_execute(&stmt.to_string()).await
    }

    pub async fn execute_stmt(
        &self,
        stmt: &impl Display,
        params: &[&ToSql],
    ) -> Result<u64, MetaStorageError> {
        self.execute(&stmt.to_string(), params).await
    }

    pub async fn query_stmt(
        &self,
        stmt: &impl Display,
        params: &[&ToSql],
    ) -> Result<Vec<tokio_postgres::Row>, MetaStorageError> {
        self.query(&stmt.to_string(), params).await
    }

    pub async fn query_opt_stmt(
        &self,
        stmt: &impl Display,
        params: &[&ToSql],
    ) -> Result<Option<tokio_postgres::Row>, MetaStorageError> {
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
