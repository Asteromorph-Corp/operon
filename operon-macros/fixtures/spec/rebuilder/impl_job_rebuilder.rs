#[operon::__private::async_trait::async_trait]
#[automatically_derived]
impl<Svc: operon::OperonService, Sto: operon::OperonStorage, MSto: operon::__private::MetaBackend> operon::__private::JobRebuilder<Svc, Sto, MSto> for BetaRebuilder {
    async fn rebuild(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), operon::error::SchedulerError<MSto::Error, Sto::Error, Svc::Error>> {
        use operon::__private::futures::{StreamExt, TryStreamExt};

        let ready_tickets = client
            .ticket(self.job_meta)
            .get_all(operon::__private::TicketStatus::Queued)
            .await?
            .into_iter()
            .map(|ticket| match ticket.resolve() {
                Some(job) => Ok(job.coordinate),
                None => Err(operon::error::SchedulerError::Other(
                    "Failed to resolve a beta ticket".into(),
                )),
            })
            .collect::<Result<std::collections::HashSet<_>, _>>()?;

        let (ready_data, invalid_data): (Vec<_>, Vec<_>) = self
            .data
            .iter()
            .cloned()
            .partition(|(job, _)| ready_tickets.contains(&job.coordinate));
        let invalid_tickets = invalid_data
            .into_iter()
            .map(|(job, _)| job)
            .collect::<Vec<_>>();

        operon::__private::futures::stream::iter(ready_data.into_iter().map(
            |(job, resolution)| async move {
                client
                    .resolution(self.spawn_dim_meta)
                    .put(resolution)
                    .await?;
                client.ticket(self.job_meta).mark_done(job).await?;

                let affected = client
                    .ticket(metadata::job_delta_meta())
                    .explode::<_, 1usize>(self.spawn_dim_meta, resolution)
                    .await?;
                for ticket in affected {
                    client
                        .ticket(metadata::job_epsilon_meta())
                        .raise_deps_quota(metadata::job_delta_meta(), ticket, &["j"], resolution.ub)
                        .await?;
                }
                client
                    .ticket(metadata::job_delta_meta())
                    .raise_deps_done(self.job_meta, job, &[])
                    .await?;
                client
                    .ticket(metadata::job_epsilon_meta())
                    .raise_deps_done(self.job_meta, job, &["j"])
                    .await?;

                let (done, queued, waiting) = client.ticket(self.job_meta).get_status().await?;
                (*self.progress.write().await).update(done, queued, waiting);

                Ok::<_, operon::error::SchedulerError<MSto::Error, Sto::Error, Svc::Error>>(())
            },
        ))
        .buffer_unordered(operon::__private::REBUILD_CONCURRENCY)
        .try_collect::<Vec<_>>()
        .await?;

        if !invalid_tickets.is_empty() {
            let count = invalid_tickets.len();
            let ticket_display = if count <= 3 {
                format!("{:?}", invalid_tickets)
            } else {
                format!(
                    "{:?}, {:?}, and {} more",
                    invalid_tickets[0],
                    invalid_tickets[1],
                    count - 2
                )
            };
            operon::__private::tracing::warn!(
                "The following {} beta ticket(s) were incorrectly marked as done: {}",
                count,
                ticket_display
            );
        }

        Ok(())
    }
}
