use std::sync::Arc;

use ex1::{
    A, B, C, CookingService, CookingStorage, D, E, F, PsqlCookingStorage, cooking_handler, schema,
};
use operon::{
    async_trait::async_trait,
    operon::{Operon, OperonOptions},
    service::OperonService,
    storage::{OperonStorage, StorageOptions},
};
use rand::Rng;

// Example minimal random service implementation
struct ExampleService;
// Implement the OperonService trait
// This must always be implemented by the user.
#[async_trait]
impl OperonService for ExampleService {
    type JobEnum = schema::JobEnum;
    type ResolutionEnum = schema::ResolutionEnum;
}

#[async_trait]
impl CookingService for ExampleService {
    async fn beta(&self, a: A) -> Result<Vec<B>, Box<dyn std::error::Error + Send + Sync>> {
        // Simulate some processing
        // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     return Err(anyhow::anyhow!("Simulated failure in beta"));
        // }
        let mut result = Vec::new();
        let mut rng = rand::rng();
        for i in 0..rng.random_range(5..=10) {
            let b = B(a.clone(), a.0.len() + i);
            result.push(b);
        }
        Ok(result)
    }

    async fn gamma(&self, a: A) -> Result<Vec<C>, Box<dyn std::error::Error + Send + Sync>> {
        // Simulate some processing
        // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     return Err(anyhow::anyhow!("Simulated failure in gamma"));
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

    async fn delta(&self, a: A, b: B, c: C) -> Result<D, Box<dyn std::error::Error + Send + Sync>> {
        // // Simulate some processing
        // // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     // panic!("Simulated failure in delta");
        //     return Err(anyhow::anyhow!("Simulated failure in delta"));
        // }
        let d = D {
            a: a.clone(),
            b: b.clone(),
            c: c.clone(),
        };
        log::trace!("Delta computed: {d:?}");
        Ok(d)
    }

    async fn epsilon(
        &self,
        b_j: Vec<B>,
        d_j: Vec<D>,
    ) -> Result<E, Box<dyn std::error::Error + Send + Sync>> {
        // Simulate some processing
        // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     return Err(anyhow::anyhow!("Simulated failure in epsilon"));
        // }
        let e = E {
            b: b_j.to_vec(),
            d: d_j.to_vec(),
        };
        Ok(e)
    }

    async fn zeta(
        &self,
        c_k: Vec<C>,
        e_k: Vec<E>,
    ) -> Result<F, Box<dyn std::error::Error + Send + Sync>> {
        // Simulate some processing
        // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     return Err(anyhow::anyhow!("Simulated failure in zeta"));
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
    let primary_ub = 1000;

    let service = Arc::new(ExampleService);

    let storage = Arc::new(PsqlCookingStorage::new(
        StorageOptions::new(&database_uri).with_schema(Some("data".to_string())),
    )?);

    let operon_options = OperonOptions::new(&database_uri);

    storage.init().await?;
    for i in 0..primary_ub {
        storage.put_a(i, A(format!("A ({i})"))).await?;
    }

    Operon::new(service, storage, operon_options)
        .run(cooking_handler(), primary_ub)
        .await?;

    Ok(())
}
