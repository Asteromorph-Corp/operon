#[allow(unused_variables)]
async fn run_job(
    &self,
    service: &Svc,
    storage: &Sto,
    meta_storage: operon::__private::MetaStorage,
    job: Self::Job,
) -> Result<Self::Resolution, operon::error::SchedulerError> {
    let [i] = job.coordinate;

    let Some(a) = storage.get_a([i]).await? else {
        return Err(
            operon::error::StorageError::EntityNotFound {
                entity: "A",
                dims: vec![("i", i)],
            }
            .into(),
        );
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

    let mut conn = meta_storage.worker_conn().await?;
    let tx = conn.transaction().await?;

    tx.as_client()
        .resolution(self.spawn_dim_meta())
        .put(resolution)
        .await?;
    tx.as_client()
        .ticket(self.job_meta())
        .mark_done(job)
        .await?;
    tx.commit().await?;

    Ok(resolution)
}
