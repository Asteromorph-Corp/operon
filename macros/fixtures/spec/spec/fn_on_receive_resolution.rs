#[allow(unused_variables, clippy::match_single_binding)]
async fn on_receive_resolution(
    &self,
    client: operon::meta_storage::MetaClient<'_>,
    peer_txs: &Self::PeerEventSenders,
    resolution: schema::ResolutionEnum,
) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
    match resolution {
        schema::ResolutionEnum::I(res) => Ok(client
            .ticket(self.job_meta())
            .explode::<_, 0usize>(metadata::dimension_i_meta(), res)
            .await?),
        schema::ResolutionEnum::J(res) => {
            match peer_txs
                .to_epsilon
                .send(operon::scheduler::PeerEvent::Explosion(
                    schema::ResolutionEnum::J(res),
                ))
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
            Ok(client
                .ticket(self.job_meta())
                .explode::<_, 1usize>(metadata::dimension_j_meta(), res)
                .await?)
        }
        schema::ResolutionEnum::K(res) => Ok(client
            .ticket(self.job_meta())
            .explode::<_, 2usize>(metadata::dimension_k_meta(), res)
            .await?),
        _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived(
            "resolution",
            "delta",
        )),
    }
}
