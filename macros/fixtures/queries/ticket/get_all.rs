pub async fn get_all_beta(
    client: operon::meta_storage::MetaClient<'_>,
    status: operon::schema_base::TicketStatus,
) -> Result<Vec<schema::BetaTicket>, operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let stmt = format!("SELECT * FROM {schema_prefix}ticket_beta WHERE status = $1;");
    let rows = client.query(&stmt, &[&status]).await?;
    let jobs = rows
        .iter()
        .map(|row| <schema::BetaTicket as operon::schema_base::TicketSql>::from_sql_row(row))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(jobs)
}
