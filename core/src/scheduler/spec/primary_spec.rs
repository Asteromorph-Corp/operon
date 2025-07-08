use async_trait::async_trait;

use crate::{
    meta_storage::MetaClient, scheduler::SchedulerError, schema_base::Resolution,
    service::OperonService, storage::OperonStorage,
};

#[async_trait]
pub trait PrimarySpec<Svc, Sto>: Clone + Send + Sync + 'static
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    type Resolution: Resolution;

    /// Run a check on the data consistency between the data storage and the metadata storage.
    /// Return `true` if the data storage holds all needed data to restore, `false` if it does not.
    async fn check_consistency(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
        primary_ub: usize,
    ) -> Result<bool, SchedulerError>; // `Scheduler::check_consistency`, 5611~
}
