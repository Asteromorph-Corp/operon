#[operon::async_trait::async_trait]
#[automatically_derived]
impl operon::schema_base::ResolutionSql for IResolution {
    async fn init_table(
        client: operon::meta_storage::MetaClient<'_>,
    ) -> Result<(), operon::meta_storage::MetaStorageError> {
        queries::init_resolution_i(client).await
    }

    async fn clear_table(
        client: operon::meta_storage::MetaClient<'_>,
    ) -> Result<(), operon::meta_storage::MetaStorageError> {
        queries::clear_resolution_i(client).await
    }

    async fn get(
        client: operon::meta_storage::MetaClient<'_>,
        primary_key: Self::PrimaryKey,
    ) -> Result<Option<Self>, operon::meta_storage::MetaStorageError> {
        queries::get_resolution_i(client).await
    }

    async fn put(
        &self,
        client: operon::meta_storage::MetaClient<'_>,
    ) -> Result<(), operon::meta_storage::MetaStorageError> {
        queries::put_resolution_i(client, self).await
    }
}
