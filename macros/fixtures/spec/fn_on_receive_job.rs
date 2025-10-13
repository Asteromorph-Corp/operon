#[allow(unused_variables, clippy::match_single_binding)]
async fn on_receive_job(
    &self,
    client: operon::meta_storage::MetaClient<'_>,
    job: schema::JobEnum,
) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
    match job {
        schema::JobEnum::Beta(job) => Ok(queries::raise_dep_epsilon(
            client,
            &operon::schema_base::TicketDepCount::some(job.i),
            &operon::schema_base::TicketDepCount::none(),
        )
        .await?),
        schema::JobEnum::Delta(job) => Ok(queries::raise_dep_epsilon(
            client,
            &operon::schema_base::TicketDepCount::some(job.i),
            &operon::schema_base::TicketDepCount::some(job.k),
        )
        .await?),
        _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived(
            "job", "epsilon",
        )),
    }
}
