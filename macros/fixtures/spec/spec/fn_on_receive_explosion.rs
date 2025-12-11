#[allow(unused_variables, clippy::match_single_binding)]
async fn on_receive_explosion(
    &self,
    client: operon::meta_storage::MetaClient<'_>,
    resolution: schema::ResolutionEnum,
    affected: usize,
) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
    match resolution {
        schema::ResolutionEnum::J(res) => Ok(client
            .ticket(self.job_meta())
            .raise_deps_quota(metadata::dimension_j_meta(), res, affected)
            .await?),
        _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived(
            "explosion",
            "epsilon",
        )),
    }
}
