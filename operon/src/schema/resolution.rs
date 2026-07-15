use std::fmt::Debug;

#[derive(Debug, Clone, Copy)]
pub struct Resolution<const N: usize> {
    pub coordinate: [usize; N],
    pub ub: usize,
}

impl<const N: usize> Resolution<N> {
    pub const fn new(ub: usize, coordinate: [usize; N]) -> Self {
        Self { coordinate, ub }
    }
}

pub trait ResolutionLike: Debug + Clone + Copy + Send + Sync + 'static {}

impl<const N: usize> ResolutionLike for Resolution<N> {}
impl ResolutionLike for () {}

pub trait ResolutionEnum: Debug + Clone + Send + Sync + 'static {}
