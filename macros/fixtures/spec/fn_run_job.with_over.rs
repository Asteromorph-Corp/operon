async fn run_job(
    &self,
    service: &Svc,
    storage: &Sto,
    client: operon::meta_storage::MetaClient<'_>,
    job: &Self::Job,
) -> Result<Self::Resolution, operon::scheduler::SchedulerError> {
    let mut resolution_j: std::collections::HashMap<(), usize> = Default::default();
    let resolution = queries::get_resolution_j(client, job.i)
        .await?
        .ok_or_else(|| {
            operon::meta_storage::MetaStorageError::MissingResolution(format!("j_{}", job.i))
        })?;
    resolution_j.insert((), resolution.0);

    let b_j = {
        let elem = storage.get_all_b_over_j(job.i).await?;
        let ub = resolution_j.get(&()).unwrap_or(&0);
        if elem.len() < *ub {
            return Err(operon::storage::StorageError::NotFound(format!(
                "b (i = {}, j = *) expects {} elements, but only {} were found",
                job.i,
                ub,
                elem.len()
            ))
            .into());
        }
        elem.into_iter()
            .take(*ub)
            .enumerate()
            .map(|(j, elem)| Ok(elem))
            .collect::<Result<Vec<_>, operon::scheduler::SchedulerError>>()
    }?;
    let d_j = {
        let elem = storage.get_all_d_over_j(job.i, job.k).await?;
        let ub = resolution_j.get(&()).unwrap_or(&0);
        if elem.len() < *ub {
            return Err(operon::storage::StorageError::NotFound(format!(
                "d (i = {}, j = *, k = {}) expects {} elements, but only {} were found",
                job.i,
                job.k,
                ub,
                elem.len()
            ))
            .into());
        }
        elem.into_iter()
            .take(*ub)
            .enumerate()
            .map(|(j, elem)| Ok(elem))
            .collect::<Result<Vec<_>, operon::scheduler::SchedulerError>>()
    }?;

    let e = service
        .epsilon(b_j, d_j)
        .await
        .map_err(operon::scheduler::SchedulerError::UserError)?;
    let resolution = ();

    storage.put_e(job.i, job.k, e).await?;
    Ok(resolution)
}
