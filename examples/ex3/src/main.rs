use operon::error::UserError;
use operon::options::{OperonOptions, StorageOptions};
use operon::{Operon, OperonService, define_operon};
use rand::Rng;

type A = ();
type B = ();
type C = ();
type D = ();
type E = ();
type F = ();
type G = ();
type H = ();

define_operon! {
    stress_test = {
        A<i> = alpha();
        B<j> = beta(A<i>, A) for i;
        C<k> = gamma(A) for i;
        D<l> = delta(A, B<i, j>, B<j>, B, C) for i, j, k;
        E<m> = epsilon(D<i, k, j, l>);
        F<n> = zeta(E, D<j, l>, D) for j, l, i, k, m;
        G = eta(F<n>, F<l, n>, F, E, D<l>, D) for n, l, i, k, m, j;
        H = theta(G, F, E, D, C, B, A) for i, j, k, l, m, n;
    }
}

#[derive(OperonService)]
struct MyService;

#[async_trait::async_trait]
impl StressTestService for MyService {
    async fn alpha(&self) -> Result<Vec<A>, UserError> {
        let mut rng = rand::rng();
        let size = rng.random_range(1..5);
        Ok((0..size).map(|_| ()).collect())
    }
    async fn beta(&self, _a_i: Vec<A>, _a: A) -> Result<Vec<B>, UserError> {
        let mut rng = rand::rng();
        let size = rng.random_range(1..5);
        Ok((0..size).map(|_| ()).collect())
    }
    async fn gamma(&self, _a: A) -> Result<Vec<C>, UserError> {
        let mut rng = rand::rng();
        let size = rng.random_range(1..5);
        Ok((0..size).map(|_| ()).collect())
    }
    async fn delta(
        &self,
        _a: A,
        _b_ij: Vec<Vec<B>>,
        _b_j: Vec<B>,
        _b: B,
        _c: C,
    ) -> Result<Vec<D>, UserError> {
        let mut rng = rand::rng();
        let size = rng.random_range(1..5);
        Ok((0..size).map(|_| ()).collect())
    }
    async fn epsilon(&self, _d_ijkl: Vec<Vec<Vec<Vec<D>>>>) -> Result<Vec<E>, UserError> {
        let mut rng = rand::rng();
        let size = rng.random_range(1..5);
        Ok((0..size).map(|_| ()).collect())
    }
    async fn zeta(&self, _e: E, _d_jl: Vec<Vec<D>>, _d: D) -> Result<Vec<F>, UserError> {
        let mut rng = rand::rng();
        let size = rng.random_range(1..5);
        Ok((0..size).map(|_| ()).collect())
    }
    async fn eta(
        &self,
        _f_n: Vec<F>,
        _f_ln: Vec<Vec<F>>,
        _f: F,
        _e: E,
        _d_l: Vec<D>,
        _d: D,
    ) -> Result<G, UserError> {
        Ok(())
    }
    async fn theta(&self, _g: G, _f: F, _e: E, _d: D, _c: C, _b: B, _a: A) -> Result<H, UserError> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_uri = std::env::var("POSTGRES_URI")?;

    let storage_options = StorageOptions::new(&database_uri).with_schema("ex3_data");
    let operon_options = OperonOptions::new(&database_uri).with_meta_storage_schema("ex3_meta");

    let service = MyService;
    let storage = PsqlStressTestStorage::new(storage_options)?;
    Operon::new(service, storage, operon_options).run().await?;

    Ok(())
}
