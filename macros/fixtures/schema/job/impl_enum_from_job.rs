#[automatically_derived]
impl From<BetaJob> for JobEnum {
    fn from(job: BetaJob) -> Self {
        JobEnum::Beta(job)
    }
}
