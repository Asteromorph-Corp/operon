#[operon::__private::async_trait::async_trait]
impl operon::OperonStorage for MemCookingStorage {
    type Error = std::convert::Infallible;

    async fn init(&self) -> operon::error::StorageResult<(), Self::Error> {
        Ok(())
    }
}
