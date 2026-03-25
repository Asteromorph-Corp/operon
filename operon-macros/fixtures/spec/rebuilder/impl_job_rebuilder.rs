#[operon::async_trait::async_trait]
#[automatically_derived]
impl operon::scheduler::JobRebuilder for BetaRebuilder {
    async fn rebuild(
        &self,
        client: operon::meta_storage::MetaClient<'_>,
    ) -> Result<(), operon::scheduler::SchedulerError> {
        let ready_tickets = client
            .ticket(self.job_meta)
            .get_all(operon::schema::TicketStatus::Queued)
            .await?
            .into_iter()
            .map(|ticket| match ticket.resolve() {
                Some(job) => Ok(job.coordinate),
                None => Err(operon::scheduler::SchedulerError::Other(
                    "Failed to resolve a beta ticket".into(),
                )),
            })
            .collect::<Result<std::collections::HashSet<_>, _>>()?;
        let mut invalid_tickets = Vec::new();

        for (job, resolution) in self.data.iter().cloned() {
            if !ready_tickets.contains(&job.coordinate) {
                invalid_tickets.push(job);
                continue;
            }
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
        }

        if !invalid_tickets.is_empty() {
            let count = invalid_tickets.len();
            let display = if count <= 3 {
                format!("{:?}", invalid_tickets)
            } else {
                format!(
                    "{:?}, {:?}, and {} more",
                    invalid_tickets[0],
                    invalid_tickets[1],
                    count - 2
                )
            };
            operon::log::warn!(
                "The following {} beta ticket(s) were incorrectly marked as done: {}",
                count,
                display
            );
        }

        Ok(())
    }
}
