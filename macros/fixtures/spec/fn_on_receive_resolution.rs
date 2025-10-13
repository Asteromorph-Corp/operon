#[allow(unused_variables, clippy::match_single_binding)]
async fn on_receive_resolution(
    &self,
    client: operon::meta_storage::MetaClient<'_>,
    resolution: schema::ResolutionEnum,
) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
    match resolution {
        schema::ResolutionEnum::I(res) => Ok(queries::explode_delta_i(client, &res).await?),
        schema::ResolutionEnum::J(res) => Ok(queries::explode_delta_j(client, &res).await?),
        schema::ResolutionEnum::K(res) => Ok(queries::explode_delta_k(client, &res).await?),
        _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived(
            "resolution",
            "delta",
        )),
    }
}
