#[automatically_derived]
impl operon::schema_base::Job for BetaJob {
    fn id() -> &'static str {
        BETA_ID
    }

    fn is_descendant_of(other: &str) -> bool {
        other == ALPHA_ID || other == BETA_ID
    }
}
