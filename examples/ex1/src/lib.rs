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

// include_operon! {
//     A<i> -- beta --> B<i, j>

//     #[entity(primary, dims = ["i"])]

//     #[entity(dims = ["i", "j"], def = "beta", from = ["A"], pool = 8)]

//     #[entity(dims = ["i", "k"], def = "gamma", from = ["A"], pool = 8)]
//     pub struct C(pub usize);

//     #[entity(
//         dims = ["i", "j", "k"],
//         def = "delta | i, j, k",
//         from = ["A", "B", "C"],
//         pool = 4
//     )]
//     pub struct D {
//         pub a: A,
//         pub b: B,
//         pub c: C,
//     }

//     B<i, j> --> epsilon
//     D<i, j, k> --> epsilon
//     epsilon --> E<i, k>

//     #[entity(dims = ["i", "k"], def = "epsilon | i, k", from = ["B | j", "D|j"], pool = 4)]
//     pub struct E {
//         pub b: Vec<B>,
//         pub d: Vec<D>,
//     }

//     #[entity(dims = ["i"], def = "zeta|i", from = ["C|k", "E|k"])]
//     pub enum F {
//         Success {
//             c: Vec<C>,
//             e: Vec<E>,
//         },
//         Failure(String, Option<C>, Option<E>),
//     }
// }

use operon::sample_operon;

sample_operon!();
