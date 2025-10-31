#[allow(unused_variables, clippy::match_single_binding)]
async fn on_receive_resolution(
    &self,
    client: operon::meta_storage::MetaClient<'_>,
    peer_txs: &Self::PeerEventSenders,
    resolution: schema::ResolutionEnum,
) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
    match resolution {
        schema::ResolutionEnum::I(res) => Ok(queries::explode_delta_i(client, &res).await?),
        schema::ResolutionEnum::J(res) => {
            match peer_txs
                .to_epsilon
                .send(operon::scheduler::PeerEvent::Explosion(res.into()))
                .await
            {
                Ok(_) => {
                    operon::log::trace!("`delta` sent peer event to `epsilon`: {resolution:?}")
                }
                Err(_) => {
                    operon::log::trace!(
                        "`epsilon`'s peer channel closed before handling `delta`'s {resolution:?}"
                    )
                }
            }
            Ok(queries::explode_delta_j(client, &res).await?)
        }
        schema::ResolutionEnum::K(res) => Ok(queries::explode_delta_k(client, &res).await?),
        _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived(
            "resolution",
            "delta",
        )),
    }
}
