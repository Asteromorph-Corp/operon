#[derive(Debug)]
pub struct BetaRebuilder {
    job_meta: operon::schema_base::JobMetadata<1usize>,
    spawn_dim_meta: operon::schema_base::DimensionMetadata<1usize>,
    data: Vec<(
        operon::schema_base::Job<1usize>,
        operon::schema_base::Resolution<1usize>,
    )>,
}
