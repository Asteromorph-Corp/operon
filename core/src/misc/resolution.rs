use std::fmt::Debug;

pub trait Resolution: Debug + Clone + Send + Sync + 'static {}

impl Resolution for () {}

pub trait ResolutionEnum: Debug + Clone + Send + Sync + 'static {
    fn primary(resolution: usize) -> Self;
}
