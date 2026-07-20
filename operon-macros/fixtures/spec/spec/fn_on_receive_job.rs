#[allow(unused_variables, clippy::match_single_binding)]
async fn on_receive_job(
    &self,
    client: MSto::Client<'_>,
    job: schema::JobEnum,
) -> Result<Vec<Self::Ticket>, operon::error::SchedulerError<MSto::Error, Sto::Error, Svc::Error>> {
    match job {
        schema::JobEnum::Beta(job) => Ok([client
            .ticket(self.job_meta())
            .raise_deps_done(metadata::job_beta_meta(), job, &["j"])
            .await?]
        .concat()),
        schema::JobEnum::Delta(job) => Ok([client
            .ticket(self.job_meta())
            .raise_deps_done(metadata::job_delta_meta(), job, &["j"])
            .await?]
        .concat()),
        _ => Err(operon::error::SchedulerError::InvalidPeerEventReceived(
            "job", "epsilon",
        )),
    }
}
