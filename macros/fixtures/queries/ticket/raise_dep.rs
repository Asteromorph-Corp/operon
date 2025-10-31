pub async fn raise_dep_beta(
    client: operon::meta_storage::MetaClient<'_>,
    i: &operon::schema_base::TicketDepCount<schema::IDim>,
) -> Result<Vec<schema::BetaTicket>, operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let params = [("i", i)]
        .into_iter()
        .filter_map(|(name, param)| param.0.map(|p| (name, p)))
        .map(|(name, param)| i64::try_from(param).map(|p| (name, p)))
        .collect::<Result<Vec<_>, _>>()?;

    let where_clause = if params.is_empty() {
        String::new()
    } else {
        let where_clause = params
            .iter()
            .enumerate()
            .map(|(i, (name, _))| format!("{} = ${}", name, i + 1))
            .collect::<Vec<_>>()
            .join(" AND ");
        format!("WHERE {where_clause}")
    };
    let params = params
        .into_iter()
        .map(|(_, param)| param)
        .collect::<Vec<_>>();
    let pop_stmt = format!(
        "DELETE FROM {schema_prefix}ticket_beta
{where_clause}
RETURNING *;"
    );

    let rows = client
        .query(
            &pop_stmt,
            &params
                .iter()
                .map(|p| p as &(dyn operon::postgres_types::ToSql + Sync))
                .collect::<Vec<_>>(),
        )
        .await?;
    let tickets = rows
        .iter()
        .map(<schema::BetaTicket as operon::schema_base::TicketSql>::from_sql_row)
        .collect::<Result<Vec<_>, _>>()?;
    let new_tickets = tickets
        .into_iter()
        .map(|ticket| operon::schema_base::Ticket::raise_dependency_count(ticket))
        .collect::<Vec<_>>();

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
