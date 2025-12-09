#[allow(unused_variables)]
async fn run_job(
    &self,
    service: &Svc,
    storage: &Sto,
    client: operon::meta_storage::MetaClient<'_>,
    job: Self::Job,
) -> Result<Self::Resolution, operon::scheduler::SchedulerError> {
    let [i] = job.coordinate;

    let mut resolution_j: std::collections::HashMap<[usize; 0usize], usize> = Default::default();
    let mut resolution_k: std::collections::HashMap<[usize; 1usize], usize> = Default::default();

    let Some(resolution) = client
        .resolution(metadata::dimension_j_meta())
        .get([i])
        .await?
    else {
        return Err(
            operon::meta_storage::MetaStorageError::MissingResolution(format!("j (i = {i})"))
                .into(),
        );
    };
    resolution_j.insert([], resolution.ub);

    for j in 0..(*resolution_j.get(&[]).unwrap_or(&0)) {
        // TODO: Remove unwrap
        let Some(resolution) = client
            .resolution(metadata::dimension_k_meta())
            .get([i, j])
            .await?
        else {
            return Err(
                operon::meta_storage::MetaStorageError::MissingResolution(format!(
                    "k (i = {i}, j = {j})",
                ))
                .into(),
            );
        };
        resolution_k.insert([j], resolution.ub);
    }

    let c_j = {
        let elem = storage.get_all_c_j([]).await?;
        let len = elem.len();
        let ub = resolution_j.get(&[]).unwrap_or(&0);
        if len < *ub {
            return Err(operon::storage::StorageError::NotFound(format!(
                "C (j = *) expects {ub} elements, but only {len} were found"
            ))
            .into());
        }
        elem.into_iter()
            .take(*ub)
            .enumerate()
            .map(|(j, elem)| Ok(elem))
            .collect::<Result<Vec<_>, operon::scheduler::SchedulerError>>()
    }?;
    let d_jk = {
        let elem = storage.get_all_d_jk([i]).await?;
        let len = elem.len();
        let ub = resolution_j.get(&[]).unwrap_or(&0);
        if len < *ub {
            return Err(operon::storage::StorageError::NotFound(format!(
                "D (i = {i}, j = *, k = _) expects {ub} elements, but only {len} were found"
            ))
            .into());
        }
        elem.into_iter()
            .take(*ub)
            .enumerate()
            .map(|(j, elem)| {
                let len = elem.len();
                let ub = resolution_k.get(&[j]).unwrap_or(&0);
                if len < *ub {
                    return Err(operon::storage::StorageError::NotFound(format!(
                        "D (i = {i}, j = {j}, k = *) expects {ub} elements, but only {len} were found"
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
        .epsilon(c_j, d_jk)
        .await
        .map_err(operon::scheduler::SchedulerError::UserError)?;
    let entity = operon::schema::Entity {
        coordinate: job.coordinate,
        value: e_l,
    };
    let resolution = operon::schema::Resolution::new(entity.value.len(), job.coordinate);

    storage.put_all_e(entity).await?;
    client
        .resolution(self.spawn_dim_meta())
        .put(resolution)
        .await?;
    Ok(resolution)
}
