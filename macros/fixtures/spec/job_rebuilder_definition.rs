#[derive(Debug)]
pub struct BetaRebuilder(Vec<(schema::BetaJob, operon::schema_base::Resolution<1usize>)>);
