async fn prepare_rebuild(
    &self,
    storage: &Sto,
    client: operon::meta_storage::MetaClient<'_>,
) -> Result<Box<dyn operon::scheduler::JobRebuilder>, operon::scheduler::SchedulerError> {
    let tickets = queries::get_all_beta(client, operon::schema_base::TicketStatus::Done).await?;
    let data =
        operon::futures::future::try_join_all(tickets.into_iter().map(|ticket| async move {
            let job = operon::schema_base::Ticket::resolve(&ticket).ok_or_else(|| {
                operon::scheduler::SchedulerError::Other("Failed to resolve a beta ticket".into())
            })?;
            let resolution = client
                .resolution(self.spawn_dim_meta())
                .get(job.primary_key)
                .await?
                .ok_or_else(|| {
                    operon::scheduler::SchedulerError::Other(format!(
                        "No resolution found for j_{:?}",
                        job.primary_key
                    ))
                })?;

            Ok::<_, operon::scheduler::SchedulerError>((job, resolution))
        }))
        .await?;

    Ok(Box::new(BetaRebuilder {
        job_meta: self.job_meta(),
        spawn_dim_meta: self.spawn_dim_meta(),
        data,
    }))
}
