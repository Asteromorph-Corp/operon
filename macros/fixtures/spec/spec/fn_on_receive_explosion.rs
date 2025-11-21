#[allow(unused_variables, clippy::match_single_binding)]
async fn on_receive_explosion(
    &self,
    client: operon::meta_storage::MetaClient<'_>,
    resolution: schema::ResolutionEnum,
) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
    match resolution {
        schema::ResolutionEnum::J(res) => Ok(queries::raise_quota_epsilon_j(client, res).await?),
        _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived(
            "explosion",
            "epsilon",
        )),
    }
}
