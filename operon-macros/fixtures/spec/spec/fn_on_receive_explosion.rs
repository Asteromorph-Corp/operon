#[allow(unused_variables, clippy::match_single_binding)]
async fn on_receive_explosion(
    &self,
    client: MSto::Client<'_>,
    explosion: operon::__private::TicketExplosion<schema::TicketEnum>,
) -> Result<Vec<Self::Ticket>, operon::error::SchedulerError<MSto::Error>> {
    match explosion.ticket {
        schema::TicketEnum::Beta(ticket) => {
            let mut out = Vec::new();
            let aggregate_dims = ["j"];
            if aggregate_dims.contains(&explosion.dim) {
                let tickets = client
                    .ticket(self.job_meta())
                    .raise_deps_quota(
                        metadata::job_beta_meta(),
                        ticket,
                        &aggregate_dims,
                        explosion.ub,
                    )
                    .await?;
                out.extend(tickets);
            }
            Ok(out)
        }
        schema::TicketEnum::Delta(ticket) => {
            let mut out = Vec::new();
            let aggregate_dims = ["j"];
            if aggregate_dims.contains(&explosion.dim) {
                let tickets = client
                    .ticket(self.job_meta())
                    .raise_deps_quota(
                        metadata::job_delta_meta(),
                        ticket,
                        &aggregate_dims,
                        explosion.ub,
                    )
                    .await?;
                out.extend(tickets);
            }
            Ok(out)
        }
        _ => Err(operon::error::SchedulerError::InvalidPeerEventReceived(
            "explosion",
            "epsilon",
        )),
    }
}
