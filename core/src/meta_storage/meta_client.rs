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
                MetaClient::Object(client) => client.$method($($arg),*).await.map_err(Into::into),
                MetaClient::Transaction(tx) => tx.$method($($arg),*).await.map_err(Into::into),
            }
        }
    };
}

type ToSql = dyn tokio_postgres::types::ToSql + Sync;

#[derive(Debug)]
pub enum MetaClient<'a> {
    Object(deadpool_postgres::Object),
    Transaction(deadpool_postgres::Transaction<'a>),
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
}

impl<'a> From<deadpool_postgres::Object> for MetaClient<'a> {
    fn from(client: deadpool_postgres::Object) -> Self {
        MetaClient::Object(client)
    }
}

impl<'a> From<deadpool_postgres::Transaction<'a>> for MetaClient<'a> {
    fn from(tx: deadpool_postgres::Transaction<'a>) -> Self {
        MetaClient::Transaction(tx)
    }
}
