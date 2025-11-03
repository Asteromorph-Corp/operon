///An enum representing any job.
#[derive(Debug, Clone)]
pub enum JobEnum {
    Alpha(AlphaJob),
    Beta(BetaJob),
    Gamma(GammaJob),
    Delta(DeltaJob),
    Epsilon(EpsilonJob),
    Zeta(ZetaJob),
}
