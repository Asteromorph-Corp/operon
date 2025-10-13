#[operon::async_trait::async_trait]
#[automatically_derived]
impl operon::schema_base::JobSql for BetaJob {
    async fn mark_done(
        &self,
        client: operon::meta_storage::MetaClient<'_>,
    ) -> Result<(), operon::meta_storage::MetaStorageError> {
        queries::mark_done_beta(client, self).await
    }
}
