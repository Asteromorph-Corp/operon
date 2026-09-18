#[operon::__private::async_trait::async_trait]
impl operon::__private::EntityQueries for CookingEntities {
    async fn init(
        &self,
        client: &operon::__private::StorageClient<'_>,
    ) -> operon::error::StorageResult<(), operon::error::PsqlStorageError> {
        self.a.init(client).await?;
        self.b.init(client).await?;
        self.c.init(client).await?;
        self.d.init(client).await?;
        self.e.init(client).await?;
        self.f.init(client).await?;
        Ok(())
    }
}
