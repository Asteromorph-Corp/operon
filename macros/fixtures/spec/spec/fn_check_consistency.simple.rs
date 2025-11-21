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
        operon::log::info!("Some `beta` tickets are corrupt in the metadata storage.");
        return Ok(false);
    };
    // ...and map them with the dimensions they spawned...
    let mut tags = Vec::new();
    for job in jobs {
        let Some(res) = client
            .resolution(self.spawn_dim_meta())
            .get(job.coordinate)
            .await?
        else {
            operon::log::info!(
                "No `j` resolution found for `beta_{:?}` in the metadata storage.",
                job.coordinate
            );
            return Ok(false);
        };
        for j in 0..(res.ub) {
            tags.push((job.coordinate, j));
        }
    }
    // ...and check if the data storage holds all the data for them.
    for ([i], j) in tags {
        if storage.get_b(i, j).await?.is_none() {
            operon::log::info!("Data storage does not hold `b_{:?}`.", [i, j]);
            return Ok(false);
        }
    }

    Ok(true)
}
