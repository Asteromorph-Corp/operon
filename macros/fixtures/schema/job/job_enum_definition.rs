///An enum representing any job.
#[derive(Debug, Clone)]
pub enum JobEnum {
    Alpha(operon::schema_base::Job<0usize>),
    Beta(operon::schema_base::Job<1usize>),
    Gamma(operon::schema_base::Job<1usize>),
    Delta(operon::schema_base::Job<3usize>),
    Epsilon(operon::schema_base::Job<2usize>),
    Zeta(operon::schema_base::Job<1usize>),
}
