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
            client
                .ticket(metadata::job_delta_meta())
                .raise_deps_done(self.job_meta, job)
                .await?;
            client
                .ticket(metadata::job_epsilon_meta())
                .raise_deps_done(self.job_meta, job)
                .await?;

            let mut ui_state = ui_state.write().await;
            let (done, queued, waiting) = client.ticket(self.job_meta).get_status().await?;
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
