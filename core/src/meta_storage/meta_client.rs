/// Wrapper around `deadpool_postgres::GenericClient` that provides
/// the `copy_in` method.
#[::async_trait::async_trait]
pub trait MetaClient: ::deadpool_postgres::GenericClient {
    async fn copy_in<T, U>(
        &self,
        query: &T,
    ) -> ::std::result::Result<::tokio_postgres::CopyInSink<U>, ::tokio_postgres::Error>
    where
        T: ?Sized + ::tokio_postgres::ToStatement + Send + Sync,
        U: ::bytes::Buf + 'static + Send + Sync;
}
#[::async_trait::async_trait]
impl MetaClient for ::deadpool_postgres::Object {
    async fn copy_in<T, U>(
        &self,
        query: &T,
    ) -> ::std::result::Result<::tokio_postgres::CopyInSink<U>, ::tokio_postgres::Error>
    where
        T: ?Sized + ::tokio_postgres::ToStatement + Send + Sync,
        U: ::bytes::Buf + 'static + Send + Sync,
    {
        ::tokio_postgres::Client::copy_in(self, query).await
    }
}
#[::async_trait::async_trait]
impl MetaClient for ::deadpool_postgres::Transaction<'_> {
    async fn copy_in<T, U>(
        &self,
        query: &T,
    ) -> ::std::result::Result<::tokio_postgres::CopyInSink<U>, ::tokio_postgres::Error>
    where
        T: ?Sized + ::tokio_postgres::ToStatement + Send + Sync,
        U: ::bytes::Buf + 'static + Send + Sync,
    {
        ::tokio_postgres::Transaction::copy_in(self, query).await
    }
}
