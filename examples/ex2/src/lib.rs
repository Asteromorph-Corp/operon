#[allow(
    unused,
    reason = "an alternative storage, swapped in by hand to time against Postgres"
)]
pub mod mem_storage;

use operon::define_operon;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct B(pub A, pub usize);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C(pub usize);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct D {
    pub a: A,
    pub b: B,
    pub c: C,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E {
    pub b: Vec<B>,
    pub d: Vec<D>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum F {
    Success { c: Vec<C>, e: Vec<E> },
    Failure(String, Option<C>, Option<E>),
}

define_operon! {
    cooking = {
        A<i> = alpha();
        B<j> = beta(A) for(8) i;
        C<k> = gamma(A) for(8) i;
        D    = delta(A, B, C) for(4) i, j, k;
        E    = epsilon(B<j>, D<j>) for(4) i, k;
        F    = zeta(C<k>, E<k>) for i;
    }
}
