#[operon::async_trait::async_trait]
#[automatically_derived]
impl operon::scheduler::JobRebuilder for BetaRebuilder {
    async fn rebuild(
        &self,
        client: operon::meta_storage::MetaClient<'_>,
        ui_state: &operon::tokio::sync::RwLock<operon::ui::UiState>,
    ) -> Result<(), operon::scheduler::SchedulerError> {
        for (job, resolution) in self.data.iter().cloned() {
            client
                .resolution(self.spawn_dim_meta)
                .put(resolution)
                .await?;
            client.ticket(self.job_meta).mark_done(job).await?;

            queries::explode_delta_j(client, resolution).await?;
            queries::raise_quota_epsilon_j(client, resolution).await?;
            queries::raise_dep_delta(
                client,
                operon::schema_base::TicketDepCount::some(job.primary_key[0usize]),
                operon::schema_base::TicketDepCount::none(),
                operon::schema_base::TicketDepCount::none(),
            )
            .await?;
            queries::raise_dep_epsilon(
                client,
                operon::schema_base::TicketDepCount::some(job.primary_key[0usize]),
                operon::schema_base::TicketDepCount::none(),
            )
            .await?;

            let mut ui_state = ui_state.write().await;
            let (done, queued, waiting) =
                <schema::BetaTicket as operon::schema_base::TicketSql>::get_status(client).await?;
            let state = if queued + waiting == 0 {
                operon::operon::RunningState::Finished
            } else {
                operon::operon::RunningState::Running
            };
            ui_state.update_ui_state(operon::ui::UiStateUpdate::ProgressUpdate(
                "beta".to_string(),
                (done, queued, waiting, state, false),
            ))?;
        }

        Ok(())
    }
}
