async fn run_job(
    &self,
    service: &Svc,
    storage: &Sto,
    client: operon::meta_storage::MetaClient<'_>,
    job: Self::Job,
) -> Result<Self::Resolution, operon::scheduler::SchedulerError> {
    let [i] = job.coordinate;

    let Some(a) = storage.get_a([i]).await? else {
        return Err(operon::storage::StorageError::NotFound(format!("a (i = {i})")).into());
    };

    let b_j = service
        .beta(a)
        .await
        .map_err(operon::scheduler::SchedulerError::UserError)?;
    let entity = operon::schema::Entity {
        coordinate: job.coordinate,
        value: b_j,
    };
    let resolution = operon::schema::Resolution::new(entity.value.len(), job.coordinate);

    storage.put_all_b(entity).await?;
    client
        .resolution(self.spawn_dim_meta())
        .put(resolution)
        .await?;
    Ok(resolution)
}
