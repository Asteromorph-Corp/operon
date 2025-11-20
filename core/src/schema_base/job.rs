use std::fmt::Debug;

#[derive(Debug, Clone, Copy)]
pub struct Job<const N: usize> {
    pub primary_key: [usize; N],
}

pub trait JobLike: Debug + Clone + Copy + Send + Sync + 'static {}

impl<const N: usize> JobLike for Job<N> {}

pub trait JobEnum: Debug + Clone + Send + Sync + 'static {}
