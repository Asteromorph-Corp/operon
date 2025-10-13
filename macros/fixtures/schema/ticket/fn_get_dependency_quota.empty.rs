#[allow(unused_variables)]
async fn get_dependency_quota(
    &self,
    client: operon::meta_storage::MetaClient<'_>,
) -> Result<Option<usize>, operon::meta_storage::MetaStorageError> {
    Ok(Some(0usize))
}
