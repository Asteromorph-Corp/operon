async fn run_job(
    &self,
    service: &Svc,
    storage: &Sto,
    client: operon::meta_storage::MetaClient<'_>,
    job: Self::Job,
) -> Result<Self::Resolution, operon::scheduler::SchedulerError> {
    let mut resolution_j: std::collections::HashMap<[usize; 0usize], usize> = Default::default();

    let pkey = [job.coordinate[0usize]];
    let Some(resolution) = client
        .resolution(metadata::dimension_j_meta())
        .get(pkey)
        .await?
    else {
        return Err(
            operon::meta_storage::MetaStorageError::MissingResolution(format!("j_{:?}", pkey))
                .into(),
        );
    };
    resolution_j.insert([], resolution.ub);

    let b_j = {
        let elem = storage.get_all_b_over_j(job.coordinate[0usize]).await?;
        let ub = resolution_j.get(&[]).unwrap_or(&0);
        if elem.len() < *ub {
            return Err(operon::storage::StorageError::NotFound(format!(
                "b (i = {}, j = *) expects {} elements, but only {} were found",
                job.coordinate[0usize],
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
        let elem = storage
            .get_all_d_over_j(job.coordinate[0usize], job.coordinate[1usize])
            .await?;
        let ub = resolution_j.get(&[]).unwrap_or(&0);
        if elem.len() < *ub {
            return Err(operon::storage::StorageError::NotFound(format!(
                "d (i = {}, j = *, k = {}) expects {} elements, but only {} were found",
                job.coordinate[0usize],
                job.coordinate[1usize],
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

    storage
        .put_e(job.coordinate[0usize], job.coordinate[1usize], e)
        .await?;
    Ok(resolution)
}
