async fn run_job(
    &self,
    service: &Svc,
    storage: &Sto,
    client: operon::meta_storage::MetaClient<'_>,
    job: &Self::Job,
) -> Result<Self::Resolution, operon::scheduler::SchedulerError> {
    let mut resolution_j: std::collections::HashMap<(), usize> = Default::default();
    let mut resolution_k: std::collections::HashMap<(schema::JDim,), usize> = Default::default();

    let resolution = queries::get_resolution_j(client, job.i)
        .await?
        .ok_or_else(|| {
            operon::meta_storage::MetaStorageError::MissingResolution(format!("j_{}", job.i))
        })?;
    resolution_j.insert((), resolution.0);

    for j in 0..(*resolution_j.get(&()).unwrap_or(&0)) {
        // TODO: Remove unwrap
        let resolution = queries::get_resolution_k(client, job.i, j)
            .await?
            .ok_or_else(|| {
                operon::meta_storage::MetaStorageError::MissingResolution(format!(
                    "k_{},{}",
                    job.i, j
                ))
            })?;
        resolution_k.insert((j,), resolution.0);
    }

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
    let d_j_k = {
        let elem = storage.get_all_d_over_jk(job.i).await?;
        let ub = resolution_j.get(&()).unwrap_or(&0);
        if elem.len() < *ub {
            return Err(operon::storage::StorageError::NotFound(format!(
                "d (i = {}, j = *, k = _) expects {} elements, but only {} were found",
                job.i,
                ub,
                elem.len()
            ))
            .into());
        }
        elem.into_iter()
            .take(*ub)
            .enumerate()
            .map(|(j, elem)| {
                let ub = resolution_k.get(&(j,)).unwrap_or(&0);
                if elem.len() < *ub {
                    return Err(operon::storage::StorageError::NotFound(format!(
                        "d (i = {}, j = {}, k = *) expects {} elements, but only {} were found",
                        job.i,
                        j,
                        ub,
                        elem.len()
                    ))
                    .into());
                }
                elem.into_iter()
                    .take(*ub)
                    .enumerate()
                    .map(|(k, elem)| Ok(elem))
                    .collect::<Result<Vec<_>, operon::scheduler::SchedulerError>>()
            })
            .collect::<Result<Vec<_>, operon::scheduler::SchedulerError>>()
    }?;

    let e_l = service
        .epsilon(b_j, d_j_k)
        .await
        .map_err(operon::scheduler::SchedulerError::UserError)?;
    let resolution = schema::LResolution(e_l.len(), job.i);

    storage.put_all_e(job.i, e_l).await?;
    Ok(resolution)
}
