use std::borrow::Cow;

use bytes::Buf;
use tokio_postgres::{CopyInSink, ToStatement};

use crate::meta_storage::MetaStorageError;

macro_rules! impl_meta_client {
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
                MetaClient::Object(ConnectionWithSchema { client, .. }) => {
                    client.$method($($arg),*).await.map_err(Into::into)
                }
                MetaClient::Transaction(TransactionWithSchema { tx, .. }) => {
                    tx.$method($($arg),*).await.map_err(Into::into)
                }
            }
        }
    };
}

type ToSql = dyn tokio_postgres::types::ToSql + Sync;

#[derive(Debug)]
pub struct ConnectionWithSchema<'a> {
    client: deadpool_postgres::Object,
    schema: Option<Cow<'a, str>>,
}

impl<'a> ConnectionWithSchema<'a> {
    pub async fn transaction(&'a mut self) -> Result<TransactionWithSchema<'a>, MetaStorageError> {
        let tx = self.client.transaction().await?;
        let schema = self.schema.as_deref().map(Cow::Borrowed);
        Ok(TransactionWithSchema { tx, schema })
    }

    pub fn as_client(&self) -> MetaClient {
        MetaClient::Object(self)
    }
}

#[derive(Debug)]
pub struct TransactionWithSchema<'a> {
    tx: deadpool_postgres::Transaction<'a>,
    schema: Option<Cow<'a, str>>,
}

impl<'a> TransactionWithSchema<'a> {
    pub async fn commit(self) -> Result<(), MetaStorageError> {
        self.tx.commit().await.map_err(Into::into)
    }

    pub async fn rollback(self) -> Result<(), MetaStorageError> {
        self.tx.rollback().await.map_err(Into::into)
    }

    pub fn as_client(&self) -> MetaClient {
        MetaClient::Transaction(self)
    }
}

#[derive(Debug)]
pub enum MetaClient<'a> {
    Object(&'a ConnectionWithSchema<'a>),
    Transaction(&'a TransactionWithSchema<'a>),
}

pub struct SchemaPrefix<'a>(Option<&'a str>);

impl std::fmt::Display for SchemaPrefix<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(schema) = self.0 {
            write!(f, "{}.", schema)
        } else {
            Ok(())
        }
    }
}

impl MetaClient<'_> {
    impl_meta_client!(batch_execute(query: &str) -> ());
    impl_meta_client!(execute(query: &str, params: &[&ToSql]) -> u64);
    impl_meta_client!(query(query: &str, params: &[&ToSql]) -> Vec<tokio_postgres::Row>);
    impl_meta_client!(query_opt(query: &str, params: &[&ToSql]) -> Option<tokio_postgres::Row>);
    impl_meta_client!(
        copy_in<T, U>(query: &T) -> CopyInSink<U>
        where T: ?Sized + ToStatement + Send + Sync,
              U: Buf + 'static + Send + Sync
    );

    pub fn schema(&self) -> Option<&str> {
        match self {
            MetaClient::Object(ConnectionWithSchema { schema, .. }) => schema.as_deref(),
            MetaClient::Transaction(TransactionWithSchema { schema, .. }) => schema.as_deref(),
        }
    }

    pub fn schema_prefix(&self) -> SchemaPrefix<'_> {
        SchemaPrefix(self.schema())
    }
}

impl<'a> From<&'a ConnectionWithSchema<'a>> for MetaClient<'a> {
    fn from(client: &'a ConnectionWithSchema<'a>) -> Self {
        MetaClient::Object(client)
    }
}

impl<'a> From<&'a TransactionWithSchema<'a>> for MetaClient<'a> {
    fn from(tx: &'a TransactionWithSchema<'a>) -> Self {
        MetaClient::Transaction(tx)
    }
}
