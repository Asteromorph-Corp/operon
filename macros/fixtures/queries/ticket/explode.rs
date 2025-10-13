pub async fn explode_beta_i(
    client: operon::meta_storage::MetaClient<'_>,
    resolution: &schema::IResolution,
) -> Result<Vec<schema::BetaTicket>, operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    // Statement to select all tickets that match the *parent dimensions* in the resolution.
    // (In this case, there are none.)
    let pop_stmt = format!("DELETE FROM {schema_prefix}ticket_beta
RETURNING *;");

    let rows = client.query(&pop_stmt, &[]).await?;
    let tickets = rows
        .iter()
        .map(<schema::BetaTicket as operon::schema_base::TicketSql>::from_sql_row)
        .collect::<Result<Vec<_>, _>>()?;

    if tickets.iter().any(|ticket| ticket.i.is_some()) {
        return Err(operon::meta_storage::MetaStorageError::InvalidExplosion(
            "Called `explode(i)` on `beta`, but `i` was resolved".into(),
        ));
    }
    let new_tickets = operon::futures::future::try_join_all(
        tickets
            .iter()
            .flat_map(|ticket| (0..resolution.0).map(|ub| ticket.clone().with_i(ub)))
            .map(|ticket| operon::schema_base::Ticket::resolve_dependency_quota(ticket, client)),
    )
    .await?;

    let copy_stmt = format!(
        "COPY {schema_prefix}ticket_beta (
    i, resolved, deps_count, deps_quota, deps_done, status
)
FROM STDIN WITH (FORMAT csv);"
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
