use async_trait::async_trait;

use crate::{
    meta_storage::MetaClient, scheduler::SchedulerError, service::OperonService,
    storage::OperonStorage,
};

#[async_trait]
pub trait PrimarySpec<Svc, Sto>: Send + Sync + 'static
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    /// Initialize the PSQL fact storage for the primary resolution.
    ///
    /// This function should be idempotent,
    /// i.e. calling it multiple times, or calling it on an already-initialized storage should do nothing.
    async fn init_resolution(&self, client: MetaClient<'_>) -> Result<(), SchedulerError>;

    /// Clear the primary resolution from the PSQL fact storage, assuming the table is already initialized.
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
