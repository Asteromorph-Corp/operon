use std::borrow::Cow;
use std::fmt::Display;

use bytes::Buf;
use deadpool_postgres::Transaction;
use tokio_postgres::{CopyInSink, ToStatement};

use crate::storage::StorageError;
use crate::utils::SchemaPrefix;

macro_rules! impl_storage_client {
    (
        $method:ident
        $(< $($generics:tt),* >)?
        ( $($arg:ident : $arg_ty:ty),* ) -> $ret:ty
        $(where $($where_clause:tt)+)?
    ) => {
        pub async fn $method
        $(< $($generics),* >)?
        (&self, $($arg: $arg_ty),*)  -> Result<$ret, StorageError>
        $(where $($where_clause)+)?
        {
            self.client.$method($($arg),*).await.map_err(Into::into)
        }
    };
}

type ToSql = dyn tokio_postgres::types::ToSql + Sync;

#[derive(Debug)]
pub struct StorageClient<'a> {
    client: deadpool_postgres::Object,
    schema: Option<Cow<'a, str>>,
}

impl<'a> StorageClient<'a> {
    pub fn new(client: deadpool_postgres::Object, schema: Option<impl Into<Cow<'a, str>>>) -> Self {
        let schema = schema.map(Into::into);
        StorageClient { client, schema }
    }

    impl_storage_client!(batch_execute(query: &str) -> ());
    impl_storage_client!(execute(query: &str, params: &[&ToSql]) -> u64);
    impl_storage_client!(query(query: &str, params: &[&ToSql]) -> Vec<tokio_postgres::Row>);
    impl_storage_client!(query_opt(query: &str, params: &[&ToSql]) -> Option<tokio_postgres::Row>);
    impl_storage_client!(
        copy_in<T, U>(query: &T) -> CopyInSink<U>
        where T: ?Sized + ToStatement + Send + Sync,
              U: Buf + 'static + Send + Sync
    );

    pub async fn batch_execute_stmt(&self, stmt: &impl Display) -> Result<(), StorageError> {
        self.batch_execute(&stmt.to_string()).await
    }

    pub async fn execute_stmt(
        &self,
        stmt: &impl Display,
        params: &[&ToSql],
    ) -> Result<u64, StorageError> {
        self.execute(&stmt.to_string(), params).await
    }

    pub async fn query_stmt(
        &self,
        stmt: &impl Display,
        params: &[&ToSql],
    ) -> Result<Vec<tokio_postgres::Row>, StorageError> {
        self.query(&stmt.to_string(), params).await
    }

    pub async fn query_opt_stmt(
        &self,
        stmt: &impl Display,
        params: &[&ToSql],
    ) -> Result<Option<tokio_postgres::Row>, StorageError> {
        self.query_opt(&stmt.to_string(), params).await
    }

    pub fn schema(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    pub fn schema_prefix(&self) -> SchemaPrefix<'_> {
        SchemaPrefix(self.schema())
    }

    pub async fn transaction(&mut self) -> Result<Transaction<'_>, StorageError> {
        let tx = self.client.transaction().await?;
        Ok(tx)
    }
}
