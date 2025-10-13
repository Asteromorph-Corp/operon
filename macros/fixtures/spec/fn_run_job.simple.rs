async fn run_job(
    &self,
    service: &Svc,
    storage: &Sto,
    client: operon::meta_storage::MetaClient<'_>,
    job: &Self::Job,
) -> Result<Self::Resolution, operon::scheduler::SchedulerError> {
    let a = storage
        .get_a(job.i)
        .await?
        .ok_or_else(|| operon::storage::StorageError::NotFound(format!("a_{}", job.i)))?;
    let b_j = service
        .beta(a)
        .await
        .map_err(operon::scheduler::SchedulerError::UserError)?;

    let resolution = schema::JResolution(b_j.len(), job.i);
    storage.put_all_b(job.i, b_j).await?;

    Ok(resolution)
}
