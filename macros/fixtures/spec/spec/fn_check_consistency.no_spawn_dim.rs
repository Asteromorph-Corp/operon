async fn check_consistency(
    &self,
    storage: &Sto,
    client: operon::meta_storage::MetaClient<'_>,
) -> Result<bool, operon::scheduler::SchedulerError> {
    let tickets = client
        .ticket(self.job_meta())
        .get_all(operon::schema::TicketStatus::Done)
        .await?;
    // Pull the "done" beta jobs from the metadata storage...
    let Some(jobs) = tickets
        .iter()
        .map(|ticket| ticket.resolve())
        .collect::<Option<Vec<_>>>()
    else {
        operon::log::info!("Some `epsilon` tickets are corrupt in the metadata storage.");
        return Ok(false);
    };
    // ...and check if the data storage holds all the data for them.
    for job in jobs {
        if storage.get_e(job.coordinate).await?.is_none() {
            operon::log::info!("Data storage does not hold `e_{:?}`.", job.coordinate);
            return Ok(false);
        }
    }

    Ok(true)
}
