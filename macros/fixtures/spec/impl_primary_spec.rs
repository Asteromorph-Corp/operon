#[operon::async_trait::async_trait]
impl<Svc: CookingService, Sto: CookingStorage> operon::scheduler::PrimarySpec<Svc, Sto>
    for PrimarySpec
{
    type Resolution = schema::IResolution;

    async fn check_consistency(
        &self,
        storage: &Sto,
        client: operon::meta_storage::MetaClient<'_>,
        primary_ub: usize,
    ) -> Result<bool, operon::scheduler::SchedulerError> {
        // Pull the primary resolution from the metadata storage..
        let Some(schema::IResolution(i_ub)) =
            <schema::IResolution as operon::schema_base::ResolutionSql>::get(client, ()).await?
        else {
            // This is technically unreachable, because we check this same value
            // in `check_recovery_state`.
            operon::log::info!("No primary resolution found in the metadata storage.");
            return Ok(false);
        };
        // ...and check if the data storage holds all the data for it.
        for i in 0..i_ub.max(primary_ub) {
            if storage.get_a(i).await?.is_none() {
                operon::log::info!("Data storage does not hold `A_{i}`.");
                return Ok(false);
            }
        }
        // Additionally check if the primary resolution agrees with the given upper bound.
        if i_ub != primary_ub {
            operon::log::warn!(
                "Previous run's upper bound `{i_ub}` is different from the current run's upper bound `{primary_ub}`.
If you overwrote the primary data, consider running `run --fresh` to overwrite the existing data, otherwise the resulting data may be inconsistent.
If you want to keep the existing data, and intendedly set the upper bound to `{primary_ub}`, you may ignore this warning."
            );
        }

        return Ok(true);
    }
}
