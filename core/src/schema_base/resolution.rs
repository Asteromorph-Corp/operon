use std::fmt::Debug;
use std::num::TryFromIntError;

use crate::utils::SqlParams;

#[derive(Debug, Clone, Copy)]
pub struct Resolution<const N: usize> {
    pub coordinate: [usize; N],
    pub ub: usize,
}

impl<const N: usize> Resolution<N> {
    pub const fn new(ub: usize, coordinate: [usize; N]) -> Self {
        Self { coordinate, ub }
    }

    pub fn as_insert_params(&self) -> Result<SqlParams, TryFromIntError> {
        SqlParams::from_usize(self.coordinate.into_iter().chain([self.ub]))
    }
}

pub trait ResolutionLike: Debug + Clone + Copy + Send + Sync + 'static {}

impl<const N: usize> ResolutionLike for Resolution<N> {}
impl ResolutionLike for () {}

pub trait ResolutionEnum: Debug + Clone + Send + Sync + 'static {}
