use std::fmt::Debug;

pub trait ResolutionEnum: Debug + Clone + Send + Sync + 'static {
    fn primary(ub: usize) -> Self;
}

pub trait Resolution: Debug + Clone + Send + Sync + 'static {}
