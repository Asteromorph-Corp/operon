#[derive(Debug)]
pub struct BetaRebuilder {
    job_meta: operon::schema::JobMetadata<1usize>,
    spawn_dim_meta: operon::schema::DimensionMetadata<1usize>,
    data: Vec<(
        operon::schema::Job<1usize>,
        operon::schema::Resolution<1usize>,
    )>,
}
