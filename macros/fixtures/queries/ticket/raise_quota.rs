pub async fn raise_quota_epsilon_j(
    client: operon::meta_storage::MetaClient<'_>,
    res: operon::schema::Resolution<1usize>,
) -> Result<Vec<operon::schema::Ticket<2usize>>, operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let pop_stmt = format!("DELETE FROM {schema_prefix}ticket_epsilon\nWHERE i = $1\nRETURNING *;");
    let rows = client
        .query(&pop_stmt, &[&i64::try_from(res.coordinate[0usize])?])
        .await?;
    let tickets = rows
        .iter()
        .map(|row| operon::schema::Ticket::from_sql_row(metadata::job_epsilon_meta(), row))
        .collect::<Result<Vec<_>, _>>()?;
    let new_tickets = tickets
        .into_iter()
        .map(|ticket| ticket.raise_deps_quota(res.ub))
        .collect::<Vec<_>>();
    let copy_stmt = format!(
        "COPY {schema_prefix}ticket_epsilon (
    i, k, deps_done, deps_quota, status
)
FROM STDIN WITH (FORMAT csv);"
    );
    let sink = client
        .copy_in::<_, operon::bytes::Bytes>(&copy_stmt)
        .await?;
    let mut sink = Box::pin(sink);
    for ticket in &new_tickets {
        operon::futures::SinkExt::feed(&mut sink, ticket.to_copy_string()?.into()).await?;
    }
    operon::futures::SinkExt::close(&mut sink).await?;
    let ready_tickets = new_tickets
        .into_iter()
        .filter(|ticket| ticket.is_ready())
        .collect::<Vec<_>>();
    Ok(ready_tickets)
}
