#[allow(unused_variables, unreachable_code, clippy::match_single_binding)]
async fn on_receive_resolution(
    &self,
    client: operon::meta_storage::MetaClient<'_>,
    peer_txs: &Self::PeerEventSenders,
    resolution: schema::ResolutionEnum,
) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
    match resolution {
        schema::ResolutionEnum::I(res) => {
            let affected = client
                .ticket(self.job_meta())
                .explode::<_, 0usize>(metadata::dimension_i_meta(), res)
                .await?;
            for ticket in affected {}
        }
        schema::ResolutionEnum::J(res) => {
            let affected = client
                .ticket(self.job_meta())
                .explode::<_, 1usize>(metadata::dimension_j_meta(), res)
                .await?;
            for ticket in affected {
                match peer_txs
                    .to_epsilon
                    .send(operon::scheduler::PeerEvent::Explosion(
                        operon::schema::TicketExplosion {
                            ticket: schema::TicketEnum::Delta(ticket),
                            dim: "j",
                            ub: res.ub,
                        },
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
            }
        }
        schema::ResolutionEnum::K(res) => {
            let affected = client
                .ticket(self.job_meta())
                .explode::<_, 2usize>(metadata::dimension_k_meta(), res)
                .await?;
            for ticket in affected {}
        }
        _ => {
            return Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived(
                "resolution",
                "delta",
            ));
        }
    }
    Ok(vec![])
}
