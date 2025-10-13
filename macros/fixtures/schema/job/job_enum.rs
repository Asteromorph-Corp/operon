///An enum representing any job.
#[derive(Debug, Clone)]
pub enum JobEnum {
    Beta(BetaJob),
    Gamma(GammaJob),
    Delta(DeltaJob),
    Epsilon(EpsilonJob),
    Zeta(ZetaJob),
}
