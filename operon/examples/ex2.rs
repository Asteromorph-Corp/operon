use async_trait::async_trait;
use clap::{Parser, Subcommand};
use operon::options::{
    MemMetaStorageOptions, MemStorageOptions, OperonOptions, PsqlMetaStorageOptions,
    PsqlStorageOptions, UiMode,
};
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
    async fn alpha(&self) -> Result<Vec<A>, Self::Error> {
        Ok((0..100).map(|i| A(format!("A ({i})"))).collect())
    }

    async fn beta(&self, a: A) -> Result<Vec<B>, Self::Error> {
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

    async fn gamma(&self, a: A) -> Result<Vec<C>, Self::Error> {
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

    async fn delta(&self, a: A, b: B, c: C) -> Result<D, Self::Error> {
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

    async fn epsilon(&self, b_j: Vec<B>, d_j: Vec<D>) -> Result<E, Self::Error> {
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

    async fn zeta(&self, c_k: Vec<C>, e_k: Vec<E>) -> Result<F, Self::Error> {
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

/// Runs the `cooking` pipeline on a storage backend chosen at startup;
/// defaults to `psql` if no backend is specified.
#[derive(Parser)]
#[command(about, long_about = None)]
struct Cli {
    /// Log to stdout instead of taking over the terminal with the interactive UI.
    #[arg(short = 'H', long)]
    headless: bool,

    #[command(subcommand)]
    backend: Option<Backend>,
}

#[derive(Subcommand)]
enum Backend {
    /// Keep entities and metadata in memory; nothing survives the run.
    Mem,
    /// Keep entities and metadata in PostgreSQL, under the `ex2_data` and `ex2_meta` schemas.
    Psql {
        /// Connection URI of the database. Read from `$POSTGRES_URI` when not given.
        #[arg(env = "POSTGRES_URI", hide_env_values = true)]
        uri: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    let service = ExampleService;
    let backend = match cli.backend {
        Some(backend) => backend,
        None => Backend::Psql {
            uri: std::env::var("POSTGRES_URI")?,
        },
    };
    let operon_options = OperonOptions::new().with_ui_mode(if cli.headless {
        UiMode::Headless
    } else {
        UiMode::Interactive
    });

    match backend {
        Backend::Mem => {
            let meta = MemMetaStorageOptions::new().build();
            let storage = MemStorageOptions::new().build();
            Operon::new(service, storage, meta)
                .with_options(operon_options)
                .run()
                .await?;
        }
        Backend::Psql { uri } => {
            let meta = PsqlMetaStorageOptions::new(&uri)
                .with_schema("ex2_meta")
                .build()?;
            let storage = PsqlStorageOptions::new(&uri)
                .with_schema("ex2_data")
                .build::<PsqlCookingStorage>()?;
            Operon::new(service, storage, meta)
                .with_options(operon_options)
                .run()
                .await?;
        }
    }

    Ok(())
}
