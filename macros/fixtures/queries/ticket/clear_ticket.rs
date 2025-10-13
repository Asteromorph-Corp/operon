pub async fn clear_ticket_beta(
    client: operon::meta_storage::MetaClient<'_>,
) -> Result<(), operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let stmt = format!("TRUNCATE TABLE {schema_prefix}ticket_beta;");
    client.execute(&stmt, &[]).await?;
    Ok(())
}
