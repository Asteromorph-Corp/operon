pub async fn put_ticket_beta(
    client: operon::meta_storage::MetaClient<'_>,
    ticket: &schema::BetaTicket,
) -> Result<(), operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let stmt = format!("INSERT INTO {schema_prefix}ticket_beta (i, resolved, deps_count, deps_quota, deps_done, status)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT DO NOTHING;");
    let params = operon::schema_base::TicketSql::to_sql_insert_params(ticket)?;
    let params = params
        .iter()
        .map(|p| p.as_ref() as &(dyn operon::postgres_types::ToSql + Sync))
        .collect::<Vec<_>>();

    client.execute(&stmt, &params).await?;
    Ok(())
}
