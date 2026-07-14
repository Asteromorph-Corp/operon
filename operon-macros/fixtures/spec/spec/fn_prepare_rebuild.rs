async fn prepare_rebuild(
    &self,
    storage: &Sto,
    progress: operon::__private::SharedProgress,
    client: MSto::Client<'_>,
) -> Result<Box<dyn operon::__private::JobRebuilder<MSto>>, operon::error::SchedulerError<MSto::Error>> {
    let tickets = client
        .ticket(self.job_meta())
        .get_all(operon::__private::TicketStatus::Done)
        .await?;
    let data = operon::__private::futures::future::try_join_all(tickets.into_iter().map(
        |ticket| async move {
            let job = ticket.resolve().ok_or_else(|| {
                operon::error::SchedulerError::Other("Failed to resolve a beta ticket".into())
            })?;
            let resolution = client
                .resolution(self.spawn_dim_meta())
                .get(job.coordinate)
                .await?
                .ok_or_else(|| {
                    operon::error::SchedulerError::Other(format!(
                        "No resolution found for j_{:?}",
                        job.coordinate
                    ))
                })?;

            Ok::<_, operon::error::SchedulerError<MSto::Error>>((job, resolution))
        },
    ))
    .await?;

    Ok(Box::new(BetaRebuilder {
        job_meta: self.job_meta(),
        spawn_dim_meta: self.spawn_dim_meta(),
        data,
        progress,
    }))
}
