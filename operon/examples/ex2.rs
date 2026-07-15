use async_trait::async_trait;
use operon::error::UserError;
use operon::options::{OperonOptions, PsqlStorageOptions};
use operon::{Operon, OperonService, define_operon};
use rand::Rng;
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

// Example service implementation
#[derive(OperonService)]
struct ExampleService;

#[async_trait]
impl CookingService for ExampleService {
    async fn alpha(&self) -> Result<Vec<A>, UserError> {
        Ok((0..100).map(|i| A(format!("A ({i})"))).collect())
    }

    async fn beta(&self, a: A) -> Result<Vec<B>, UserError> {
        // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     return Err("Simulated failure in beta".into());
        // }
        let mut result = Vec::new();
        let mut rng = rand::rng();
        for i in 0..rng.random_range(5..=10) {
            let b = B(a.clone(), a.0.len() + i);
            result.push(b);
        }
        Ok(result)
    }

    async fn gamma(&self, a: A) -> Result<Vec<C>, UserError> {
        // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     return Err("Simulated failure in gamma".into());
        // }
        let mut result = Vec::new();
        let mut rng = rand::rng();
        let len = a.0.len();
        for _ in 0..rng.random_range(5..=10) {
            let c = C(rng.random_range(1..=len));
            result.push(c);
        }
        Ok(result)
    }

    async fn delta(&self, a: A, b: B, c: C) -> Result<D, UserError> {
        // // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     // panic!("Simulated failure in delta");
        //     return Err("Simulated failure in delta".into());
        // }
        let d = D {
            a: a.clone(),
            b: b.clone(),
            c: c.clone(),
        };
        // Use the `log` crate inside jobs
        log::trace!("Delta computed: {d:?}");
        Ok(d)
    }

    async fn epsilon(&self, b_j: Vec<B>, d_j: Vec<D>) -> Result<E, UserError> {
        // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     return Err("Simulated failure in epsilon".into());
        // }
        let e = E {
            b: b_j.to_vec(),
            d: d_j.to_vec(),
        };
        Ok(e)
    }

    async fn zeta(&self, c_k: Vec<C>, e_k: Vec<E>) -> Result<F, UserError> {
        // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     return Err("Simulated failure in zeta".into());
        // }
        let mut rng = rand::rng();
        let f = if rng.random_bool(0.5) {
            F::Success {
                c: c_k.to_vec(),
                e: e_k.to_vec(),
            }
        } else {
            F::Failure(
                format!("F (failure with {} C and {} E)", c_k.len(), e_k.len()),
                c_k.to_vec().first().cloned(),
                e_k.to_vec().first().cloned(),
            )
        };
        Ok(f)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let database_uri = std::env::var("POSTGRES_URI")?;

    let storage_options = PsqlStorageOptions::new(&database_uri).with_schema("ex2_data");
    let operon_options = OperonOptions::new(&database_uri).with_meta_storage_schema("ex2_meta");

    let service = ExampleService;
    let storage = PsqlCookingStorage::new(storage_options)?;
    Operon::new(service, storage, operon_options).run().await?;

    Ok(())
}
