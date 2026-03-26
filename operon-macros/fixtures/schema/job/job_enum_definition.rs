/// An enum representing any job.
#[derive(Debug, Clone)]
pub enum JobEnum {
    Alpha(operon::__private::Job<0usize>),
    Beta(operon::__private::Job<1usize>),
    Gamma(operon::__private::Job<1usize>),
    Delta(operon::__private::Job<3usize>),
    Epsilon(operon::__private::Job<2usize>),
    Zeta(operon::__private::Job<1usize>),
}
