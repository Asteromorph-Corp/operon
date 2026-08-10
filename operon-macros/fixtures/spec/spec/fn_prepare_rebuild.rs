async fn prepare_rebuild(
    &self,
    storage: &Sto,
    progress: operon::__private::SharedProgress,
    client: MSto::Client<'_>,
) -> Result<Box<dyn operon::__private::TaskRebuilder<Svc, Sto, MSto>>, operon::error::SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
    use operon::__private::futures::{StreamExt, TryStreamExt};

    let tickets = client
        .ticket(self.task_meta())
        .get_all(operon::__private::TicketStatus::Done)
        .await?;
    let data = operon::__private::futures::stream::iter(tickets.into_iter().map(
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

            Ok::<_, operon::error::SchedulerError<Svc::Error, Sto::Error, MSto::Error>>((job, resolution))
        },
    ))
    .buffered(operon::__private::REBUILD_CONCURRENCY)
    .try_collect::<Vec<_>>()
    .await?;

    Ok(Box::new(BetaRebuilder {
        task_meta: self.task_meta(),
        spawn_dim_meta: self.spawn_dim_meta(),
        data,
        progress,
    }))
}
