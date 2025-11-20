async fn run_job(
    &self,
    service: &Svc,
    storage: &Sto,
    client: operon::meta_storage::MetaClient<'_>,
    job: Self::Job,
) -> Result<Self::Resolution, operon::scheduler::SchedulerError> {
    let Some(a) = storage.get_a(job.primary_key[0usize]).await? else {
        return Err(operon::storage::StorageError::NotFound(format!(
            "a_{}",
            job.primary_key[0usize]
        ))
        .into());
    };
    let b_j = service
        .beta(a)
        .await
        .map_err(operon::scheduler::SchedulerError::UserError)?;

    let resolution = operon::schema_base::Resolution::new(b_j.len(), job.primary_key);
    storage.put_all_b(job.primary_key[0usize], b_j).await?;
    client
        .resolution(self.spawn_dim_meta())
        .put(resolution)
        .await?;
    Ok(resolution)
}
