#[derive(Debug)]
pub struct BetaRebuilder {
    job_meta: operon::__private::JobMetadata<1usize>,
    spawn_dim_meta: operon::__private::DimensionMetadata<1usize>,
    data: Vec<(
        operon::__private::Job<1usize>,
        operon::__private::Resolution<1usize>,
    )>,
    progress: operon::__private::SharedProgress,
}
