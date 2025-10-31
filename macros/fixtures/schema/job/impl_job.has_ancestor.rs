#[automatically_derived]
impl operon::schema_base::Job for EpsilonJob {
    fn id() -> &'static str {
        EPSILON_ID
    }

    fn is_descendant_of(other: &str) -> bool {
        other == ALPHA_ID
            || other == BETA_ID
            || other == DELTA_ID
            || other == EPSILON_ID
            || other == GAMMA_ID
    }
}
