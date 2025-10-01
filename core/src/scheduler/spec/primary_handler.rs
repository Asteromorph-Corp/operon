use async_trait::async_trait;

use crate::meta_storage::MetaClient;
use crate::scheduler::{PrimarySpec, SchedulerError};
use crate::schema_base::ResolutionSql;
use crate::service::OperonService;
use crate::storage::OperonStorage;

#[async_trait]
pub trait PrimaryHandler<Svc, Sto>: Send + Sync + 'static
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    /// Initialize the PSQL fact storage for the primary resolution.
    ///
    /// This function should be idempotent,
    /// i.e. calling it multiple times, or calling it on an already-initialized storage should do
    /// nothing.
    async fn init_resolution(&self, client: MetaClient<'_>) -> Result<(), SchedulerError>;

    /// Clear the primary resolution from the PSQL fact storage, assuming the table is already
    /// initialized.
    async fn clear_resolution(&self, client: MetaClient<'_>) -> Result<(), SchedulerError>; // `facts_psql::clear`, 877~

    /// Get the primary resolution from the PSQL fact storage.
    async fn get_primary_resolution(
        &self,
        client: MetaClient<'_>,
    ) -> Result<Option<usize>, SchedulerError>; // `facts_psql::get_resolution`, 891~

    /// Put a primary resolution into the PSQL fact storage.
    async fn put_primary_resolution(
        &self,
        client: MetaClient<'_>,
        ub: usize,
    ) -> Result<(), SchedulerError>;

    /// Run a check on the data consistency between the data storage and the metadata storage.
    /// Return `true` if the data storage holds all needed data to restore, `false` if it does not.
    async fn check_consistency(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
        primary_ub: usize,
    ) -> Result<bool, SchedulerError>; // `Scheduler::check_consistency`, 5611~
}

#[async_trait]
impl<Svc, Sto, R, PS> PrimaryHandler<Svc, Sto> for PS
where
    Svc: OperonService,
    Sto: OperonStorage,
    R: ResolutionSql<PrimaryKey = ()>,
    PS: PrimarySpec<Svc, Sto, Resolution = R>,
{
    async fn init_resolution(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        R::init_table(client).await?;
        Ok(())
    }

    async fn clear_resolution(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        R::clear_table(client).await?;
        Ok(())
    }

    async fn get_primary_resolution(
        &self,
        client: MetaClient<'_>,
    ) -> Result<Option<usize>, SchedulerError> {
        let resolution = R::get(client, ()).await?;
        Ok(resolution.map(|r| r.ub()))
    }

    async fn put_primary_resolution(
        &self,
        client: MetaClient<'_>,
        ub: usize,
    ) -> Result<(), SchedulerError> {
        let resolution = R::new(ub, ());
        resolution.put(client).await?;
        Ok(())
    }

    async fn check_consistency(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
        primary_ub: usize,
    ) -> Result<bool, SchedulerError> {
        PrimarySpec::check_consistency(self, storage, client, primary_ub).await
    }
}
