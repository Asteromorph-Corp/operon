async fn check_consistency(
    &self,
    storage: &Sto,
    client: operon::meta_storage::MetaClient<'_>,
) -> Result<bool, operon::scheduler::SchedulerError> {
    // Pull the "done" beta jobs from the metadata storage...
    let Some(jobs) = queries::get_all_epsilon(client, operon::schema_base::TicketStatus::Done)
        .await?
        .iter()
        .map(|t| operon::schema_base::Ticket::resolve(t))
        .collect::<Option<Vec<_>>>()
    else {
        operon::log::info!("Some `epsilon` tickets are corrupt in the metadata storage.");
        return Ok(false);
    };
    // ...and check if the data storage holds all the data for them.
    for job in jobs {
        let [i, k] = job.coordinate;
        if storage.get_e(i, k).await?.is_none() {
            operon::log::info!("Data storage does not hold `e_{:?}`.", [i, k]);
            return Ok(false);
        }
    }

    Ok(true)
}
