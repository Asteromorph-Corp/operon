use operon::serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "operon::serde")]
pub struct A(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "operon::serde")]
pub struct B(pub A, pub usize);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "operon::serde")]
pub struct C(pub usize);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "operon::serde")]
pub struct D {
    pub a: A,
    pub b: B,
    pub c: C,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "operon::serde")]
pub struct E {
    pub b: Vec<B>,
    pub d: Vec<D>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "operon::serde")]
pub enum F {
    Success { c: Vec<C>, e: Vec<E> },
    Failure(String, Option<C>, Option<E>),
}

use operon::sample_operon;

sample_operon! {
    Cooking = |A<i>| {
        B<j> = beta(A) for(8) i;
        C<k> = gamma(A) for(8) i;
        D    = delta(A, B, C) for(4) i, j, k;
        E    = epsilon(B<j>, D<j>) for(4) i, k;
        F    = zeta(C<k>, E<k>) for i;
    }
}
