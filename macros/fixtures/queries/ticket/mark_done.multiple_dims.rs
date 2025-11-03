pub async fn mark_done_epsilon(
    client: operon::meta_storage::MetaClient<'_>,
    job: &schema::EpsilonJob,
) -> Result<(), operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let stmt = format!(
        "UPDATE {schema_prefix}ticket_epsilon SET status = 'done' WHERE i = $1 AND k = $2;"
    );
    client
        .execute(&stmt, &[&i64::try_from(job.i)?, &i64::try_from(job.k)?])
        .await?;
    Ok(())
}

