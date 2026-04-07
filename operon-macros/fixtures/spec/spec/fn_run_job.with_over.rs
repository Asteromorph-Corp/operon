#[allow(unused_variables)]
async fn run_job(
    &self,
    service: &Svc,
    storage: &Sto,
    client: operon::__private::MetaClient<'_>,
    job: Self::Job,
) -> Result<Self::Resolution, operon::error::SchedulerError> {
    let [i, k] = job.coordinate;

    let mut resolution_j: std::collections::HashMap<[usize; 0usize], usize> = Default::default();

    let Some(resolution) = client
        .resolution(metadata::dimension_j_meta())
        .get([i])
        .await?
    else {
        return Err(
            operon::error::MetaStorageError::MissingResolution {
                dim: "j",
                deps: vec![("i", i)],
            }
            .into(),
        );
    };
    resolution_j.insert([], resolution.ub);

    let b_j = {
        let elem = storage.get_all_b_j([i]).await?;
        let len = elem.len();
        let ub = resolution_j.get(&[]).unwrap_or(&0);
        if len < *ub {
            return Err(operon::error::StorageError::NotFound(format!(
                "B (i = {i}, j = *) expects {ub} elements, but only {len} were found"
            ))
            .into());
        }
        elem.into_iter()
            .take(*ub)
            .enumerate()
            .map(|(j, elem)| Ok(elem))
            .collect::<Result<Vec<_>, operon::error::SchedulerError>>()
    }?;
    let d_j = {
        let elem = storage.get_all_d_j([i, k]).await?;
        let len = elem.len();
        let ub = resolution_j.get(&[]).unwrap_or(&0);
        if len < *ub {
            return Err(operon::error::StorageError::NotFound(format!(
                "D (i = {i}, j = *, k = {k}) expects {ub} elements, but only {len} were found"
            ))
            .into());
        }
        elem.into_iter()
            .take(*ub)
            .enumerate()
            .map(|(j, elem)| Ok(elem))
            .collect::<Result<Vec<_>, operon::error::SchedulerError>>()
    }?;

    let e = service
        .epsilon(b_j, d_j)
        .await
        .map_err(operon::error::SchedulerError::UserError)?;
    let entity = operon::Entity {
        coordinate: job.coordinate,
        value: e,
    };
    let resolution = ();

    storage.put_e(entity).await?;
    Ok(resolution)
}
