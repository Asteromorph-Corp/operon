pub async fn mark_done_beta(
    client: operon::meta_storage::MetaClient<'_>,
    job: &schema::BetaJob,
) -> Result<(), operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let stmt = format!("UPDATE {schema_prefix}ticket_beta SET status = 'done' WHERE i = $1;");
    client.execute(&stmt, &[&i64::try_from(job.i)?]).await?;
    Ok(())
}
