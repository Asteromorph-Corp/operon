/// An enum representing any job.
#[derive(Debug, Clone)]
pub enum JobEnum {
    Alpha(operon::schema::Job<0usize>),
    Beta(operon::schema::Job<1usize>),
    Gamma(operon::schema::Job<1usize>),
    Delta(operon::schema::Job<3usize>),
    Epsilon(operon::schema::Job<2usize>),
    Zeta(operon::schema::Job<1usize>),
}
