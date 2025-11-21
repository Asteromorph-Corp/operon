pub async fn explode_beta_i(
    client: operon::meta_storage::MetaClient<'_>,
    resolution: operon::schema_base::Resolution<0usize>,
) -> Result<Vec<operon::schema_base::Ticket<1usize>>, operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    // Statement to select all tickets that match the *parent dimensions* in the resolution.
    // (In this case, there are none.)
    let pop_stmt = format!(
        "DELETE FROM {schema_prefix}ticket_beta
RETURNING *;"
    );

    let rows = client.query(&pop_stmt, &[]).await?;
    let tickets = rows
        .iter()
        .map(|row| operon::schema_base::Ticket::from_sql_row(metadata::job_beta_meta(), row))
        .collect::<Result<Vec<_>, _>>()?;

    if tickets
        .iter()
        .any(|ticket| ticket.coordinate[0usize].is_some())
    {
        return Err(operon::meta_storage::MetaStorageError::InvalidExplosion(
            "Called `explode(i)` on `beta`, but `i` was resolved".into(),
        ));
    }
    let new_tickets = tickets
        .iter()
        .flat_map(|ticket| (0..resolution.ub).map(|x| ticket.clone().with_coordinate::<0usize>(x)))
        .map(|ticket| ticket.update_status())
        .collect::<Vec<_>>();

    let copy_stmt = format!(
        "COPY {schema_prefix}ticket_beta (
    i, deps_done, deps_quota, status
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
