pub async fn raise_quota_epsilon_j(
    client: operon::meta_storage::MetaClient<'_>,
    res: operon::schema_base::Resolution<1usize>,
) -> Result<Vec<schema::EpsilonTicket>, operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let pop_stmt = format!("DELETE FROM {schema_prefix}ticket_epsilon\nWHERE i = $1\nRETURNING *;");
    let rows = client
        .query(&pop_stmt, &[&i64::try_from(res.primary_key[0usize])?])
        .await?;
    let tickets = rows
        .iter()
        .map(<schema::EpsilonTicket as operon::schema_base::TicketSql>::from_sql_row)
        .collect::<Result<Vec<_>, _>>()?;
    let new_tickets = tickets
        .into_iter()
        .map(|ticket| operon::schema_base::Ticket::raise_dependency_quota(ticket, res.ub))
        .collect::<Vec<_>>();
    let copy_stmt = format!(
        "COPY {schema_prefix}ticket_epsilon (\n    i, k, resolved, deps_count, deps_quota, deps_done, status\n)\nFROM STDIN WITH (FORMAT csv);"
    );
    let sink = client
        .copy_in::<_, operon::bytes::Bytes>(&copy_stmt)
        .await?;
    let mut sink = Box::pin(sink);
    for ticket in &new_tickets {
        operon::futures::SinkExt::feed(
            &mut sink,
            operon::schema_base::TicketSql::to_sql_copy_params(ticket)?.into(),
        )
        .await?;
    }
    operon::futures::SinkExt::close(&mut sink).await?;
    let ready_tickets = new_tickets
        .into_iter()
        .filter(operon::schema_base::Ticket::is_ready)
        .collect::<Vec<_>>();
    Ok(ready_tickets)
}
