#[allow(unused_variables)]
async fn get_dependency_quota(
    &self,
    client: operon::meta_storage::MetaClient<'_>,
) -> Result<Option<usize>, operon::meta_storage::MetaStorageError> {
    let Some(i) = self.i.0 else {
        return Ok(None);
    };

    let mut resolution_j: std::collections::HashMap<(), usize> = Default::default();

    if let Some(resolution) = queries::get_resolution_j(client, i).await? {
        resolution_j.insert((), resolution.0);
    } else {
        return Ok(None);
    }

    Ok(Some(1usize + resolution_j.values().sum::<usize>() * 1usize))
}
