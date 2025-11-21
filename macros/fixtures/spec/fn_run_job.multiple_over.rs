async fn run_job(
    &self,
    service: &Svc,
    storage: &Sto,
    client: operon::meta_storage::MetaClient<'_>,
    job: Self::Job,
) -> Result<Self::Resolution, operon::scheduler::SchedulerError> {
    let mut resolution_j: std::collections::HashMap<[usize; 0usize], usize> = Default::default();
    let mut resolution_k: std::collections::HashMap<[usize; 1usize], usize> = Default::default();

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

    for j in 0..(*resolution_j.get(&[]).unwrap_or(&0)) {
        // TODO: Remove unwrap
        let pkey = [job.coordinate[0usize], j];
        let Some(resolution) = client
            .resolution(metadata::dimension_k_meta())
            .get(pkey)
            .await?
        else {
            return Err(
                operon::meta_storage::MetaStorageError::MissingResolution(format!("k_{:?}", pkey))
                    .into(),
            );
        };
        resolution_k.insert([j], resolution.ub);
    }

    let c_j = {
        let elem = storage.get_all_c_over_j().await?;
        let ub = resolution_j.get(&[]).unwrap_or(&0);
        if elem.len() < *ub {
            return Err(operon::storage::StorageError::NotFound(format!(
                "c (j = *) expects {} elements, but only {} were found",
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
        let elem = storage.get_all_d_over_jk(job.coordinate[0usize]).await?;
        let ub = resolution_j.get(&[]).unwrap_or(&0);
        if elem.len() < *ub {
            return Err(operon::storage::StorageError::NotFound(format!(
                "d (i = {}, j = *, k = _) expects {} elements, but only {} were found",
                job.coordinate[0usize],
                ub,
                elem.len()
            ))
            .into());
        }
        elem.into_iter()
            .take(*ub)
            .enumerate()
            .map(|(j, elem)| {
                let ub = resolution_k.get(&[j]).unwrap_or(&0);
                if elem.len() < *ub {
                    return Err(operon::storage::StorageError::NotFound(format!(
                        "d (i = {}, j = {}, k = *) expects {} elements, but only {} were found",
                        job.coordinate[0usize],
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
        .epsilon(c_j, d_j_k)
        .await
        .map_err(operon::scheduler::SchedulerError::UserError)?;
    let resolution = operon::schema_base::Resolution::new(e_l.len(), job.coordinate);

    storage.put_all_e(job.coordinate[0usize], e_l).await?;
    client
        .resolution(self.spawn_dim_meta())
        .put(resolution)
        .await?;
    Ok(resolution)
}
