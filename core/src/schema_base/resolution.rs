use std::fmt::Debug;
use std::num::TryFromIntError;

use crate::utils::SqlParams;

#[derive(Debug, Clone, Copy)]
pub struct Resolution<const N: usize> {
    pub primary_key: [usize; N],
    pub ub: usize,
}

impl<const N: usize> Resolution<N> {
    pub const fn new(ub: usize, primary_key: [usize; N]) -> Self {
        Self { primary_key, ub }
    }

    pub fn as_insert_params(&self) -> Result<SqlParams, TryFromIntError> {
        SqlParams::from_usize(self.primary_key.into_iter().chain([self.ub]))
    }
}

pub trait ResolutionEnum: Debug + Clone + Send + Sync + 'static {}
