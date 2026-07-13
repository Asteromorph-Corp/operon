#[allow(unused_variables)]
async fn run_job(
    &self,
    service: &Svc,
    storage: &Sto,
    meta_storage: MSto,
    job: Self::Job,
) -> Result<Self::Resolution, operon::error::SchedulerError> {
    let [i] = job.coordinate;

    let (resolution_j, resolution_k_j) = {    
        let mut resolution_j: std::collections::HashMap<[usize; 0usize], usize> = Default::default();
        let mut resolution_k_j: std::collections::HashMap<[usize; 1usize], usize> = Default::default();
        
        let conn = meta_storage.worker_conn().await?;
        let client = conn.as_client();

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

        for j in 0..(*resolution_j.get(&[]).unwrap_or(&0)) {
            // TODO: Remove unwrap
            let Some(resolution) = client
                .resolution(metadata::dimension_k_meta())
                .get([i, j])
                .await?
            else {
                return Err(
                    operon::error::MetaStorageError::MissingResolution {
                        dim: "k",
                        deps: vec![("i", i), ("j", j)],
                    }
                    .into(),
                );
            };
            resolution_k_j.insert([j], resolution.ub);
        }

        (resolution_j, resolution_k_j)
    };

    let c_j = {
        let elem = storage.get_all_c_j([]).await?;
        let len = elem.len();
        let ub = resolution_j.get(&[]).unwrap_or(&0);
        if len < *ub {
            return Err(
                operon::error::StorageError::EntityLengthMismatch {
                    entity: "C",
                    dims: vec![("j", operon::error::DimState::Aggregated)],
                    expected: *ub,
                    actual: len,
                }
                .into(),
            );
        }
        elem.into_iter()
            .take(*ub)
            .enumerate()
            .map(|(j, elem)| Ok(elem))
            .collect::<Result<Vec<_>, operon::error::SchedulerError>>()
    }?;
    let d_jk = {
        let elem = storage.get_all_d_jk([i]).await?;
        let len = elem.len();
        let ub = resolution_j.get(&[]).unwrap_or(&0);
        if len < *ub {
            return Err(
                operon::error::StorageError::EntityLengthMismatch {
                    entity: "D",
                    dims: vec![
                        ("i", operon::error::DimState::Value(i)),
                        ("j", operon::error::DimState::Aggregated),
                        ("k", operon::error::DimState::Unresolved),
                    ],
                    expected: *ub,
                    actual: len,
                }
                .into(),
            );
        }
        elem.into_iter()
            .take(*ub)
            .enumerate()
            .map(|(j, elem)| {
                let len = elem.len();
                let ub = resolution_k_j.get(&[j]).unwrap_or(&0);
                if len < *ub {
                    return Err(
                        operon::error::StorageError::EntityLengthMismatch {
                            entity: "D",
                            dims: vec![
                                ("i", operon::error::DimState::Value(i)),
                                ("j", operon::error::DimState::Value(j)),
                                ("k", operon::error::DimState::Aggregated),
                            ],
                            expected: *ub,
                            actual: len,
                        }
                        .into(),
                    );
                }
                elem.into_iter()
                    .take(*ub)
                    .enumerate()
                    .map(|(k, elem)| Ok(elem))
                    .collect::<Result<Vec<_>, operon::error::SchedulerError>>()
            })
            .collect::<Result<Vec<_>, operon::error::SchedulerError>>()
    }?;

    let e_l = service
        .epsilon(c_j, d_jk)
        .await
        .map_err(operon::error::SchedulerError::UserError)?;
    let entity = operon::Entity {
        coordinate: job.coordinate,
        value: e_l,
    };
    let resolution = operon::__private::Resolution::new(entity.value.len(), job.coordinate);

    storage.put_all_e(entity).await?;

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
