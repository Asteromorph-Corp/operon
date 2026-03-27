#[allow(unused_variables)]
async fn run_job(
    &self,
    service: &Svc,
    storage: &Sto,
    client: operon::__private::MetaClient<'_>,
    job: Self::Job,
) -> Result<Self::Resolution, operon::error::SchedulerError> {
    let [i] = job.coordinate;

    let Some(a) = storage.get_a([i]).await? else {
        return Err(operon::error::StorageError::NotFound(format!("A (i = {i})")).into());
    };

    let b_j = service
        .beta(a)
        .await
        .map_err(operon::error::SchedulerError::UserError)?;
    let entity = operon::Entity {
        coordinate: job.coordinate,
        value: b_j,
    };
    let resolution = operon::__private::Resolution::new(entity.value.len(), job.coordinate);

    storage.put_all_b(entity).await?;
    client
        .resolution(self.spawn_dim_meta())
        .put(resolution)
        .await?;
    Ok(resolution)
}
