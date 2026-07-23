//! # Operon Example 1: Splitter
//!
//! This example code serves as a basic introduction to Operon.
//! Here, we demonstrate a simple use case of Operon,
//! where we split a string into words and then into characters.
//!
//! To run this example as-is, you will need an URI to a working PostgreSQL database,
//! and use the following shell command:
//!
//! ```bash
//! POSTGRES_URI=<database_uri> cargo run --release --example ex1
//! ```
//!
//! This example is meant to be run as a binary,
//! and was tested with Rust 1.91.1 and PostgreSQL 16+.

use std::sync::Arc;

use async_trait::async_trait;
use operon::options::{OperonOptions, PsqlMetaStorageOptions, PsqlStorageOptions};
use operon::{Operon, OperonService, PsqlMetaStorage, define_operon};
use serde::{Deserialize, Serialize};

//# —————————————————————— A. Entity Definitions —————————————————————— #//
// Define the entities that will be used.
// Entities must implement the following traits:
// * `Clone`
// * `Debug`
// * Optionally, `Serialize` and `Deserialize` for database usage.

// Strings already implement all the necessary traits,
// so using a type alias of `String` is sufficient for our `Input` type.
type Input = String;

// For composite types, we need to implement or derive the necessary traits.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Intermediate(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Output(char);
//# ——————————————————————————————————————————————————————————————————— #//

//# —————————————————————— B. Pipeline Definition ————————————————————— #//
// Define the pipeline.
// More information about the macro DSL can be found in the documentations.
//
// This example pipeline is called "splitter", and it defines a simple flow:
// ┌─────────────────────────────────────────────────────────────┐
// │ Input  ——get_words——>  Intermediate  ——get_chars——>  Output │
// └─────────────────────────────────────────────────────────────┘
// where `Input`s are indexed by `[input_no]`,
// `Intermediate`s are indexed by `[input_no][word_no]`,
// and `Output`s are indexed by `[input_no][word_no][char_no]`.
//
// Since our system using named index dimensions may be nontrivial,
// we provide detailed documentation for the semantic system in this repository.

define_operon! {
    splitter = {
        Input<input_no> = get_inputs();
        Intermediate<word_no> = get_words(Input) for input_no;
        Output<char_no> = get_chars(Intermediate) for input_no, word_no;
    }
}
//# ——————————————————————————————————————————————————————————————————— #//

//# ———————————————————— C. Service Implementation ———————————————————— #//
// Define the behaviour of each step in the pipeline.

// The necessary functions are generated as traits,
// so we will need to create a struct to implement them.
// The struct must derive `OperonService`.
// Since this example is infallible, we denote the error type as such.
#[derive(OperonService)]
#[operon(error = std::convert::Infallible)]
struct MySplitterService;

// The following is the main trait that we need to implement.
// Note that its name depends on the pipeline name we gave above.
// Consult the generated documentation for this trait
// to see what methods are required.
#[async_trait]
impl SplitterService for MySplitterService {
    async fn get_inputs(&self) -> Result<Vec<Input>, Self::Error> {
        Ok(vec![
            Input::from("Hello World"),
            Input::from("Hello Operon"),
            Input::from(""),
        ])
    }

    async fn get_words(&self, input: Input) -> Result<Vec<Intermediate>, Self::Error> {
        Ok(input
            .split_whitespace()
            .map(|s| Intermediate(s.to_string()))
            .collect())
    }

    async fn get_chars(&self, intermediate: Intermediate) -> Result<Vec<Output>, Self::Error> {
        // To print something to the UI,
        // we can use the `log` crate directly,
        // Do not write to `stdout` or `stderr` directly,
        // as it will interfere with the Operon UI.
        log::info!("Processing intermediate: {}", intermediate.0);
        Ok(intermediate.0.chars().map(Output).collect())
    }
}
//# ——————————————————————————————————————————————————————————————————— #//

//# ————————————————————————— D. Running Operon ——————————————————————— #//
// Run the Operon service for the above pipeline.
// Always run this as a binary.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // We expect the database URI to be passed as an env variable here,
    // but we could also hardcode the URI (not recommended for production)
    // or use some other method like `.env`-`dotenvy` to load it.
    let database_uri = std::env::var("POSTGRES_URI")?;

    // The run settings.
    let operon_options = OperonOptions::new();

    //# ——————————————————— Initializing Settings ————————————————————— #//
    // The service is what we defined above —
    // a struct that holds the logic for the pipeline.
    let service = Arc::new(MySplitterService);

    // The storage is where all the data is stored and retrieved.
    // Here, we use `PsqlSplitterStorage` for the storage.
    // This is an automatically generated storage implementation
    // based on a PostgreSQL database.
    let storage = Arc::new(
        PsqlStorageOptions::new(&database_uri)
            .with_schema("ex1_data")
            .build::<PsqlSplitterStorage>()?,
    );

    // The metadata backend, built and handed to Operon.
    let meta = PsqlMetaStorageOptions::new(&database_uri)
        .with_schema("ex1_meta")
        .build()?;

    //# ———————————————————————— Running Operon ——————————————————————— #//
    // Now we finally run Operon.

    // Initialize an Operon struct with the configurations above.
    // We cloned the `storage` Arc because we intend to access the storage
    // after the Operon instance has finished running,
    // but this is entirely optional.
    let operon_instance: Operon<MySplitterService, PsqlSplitterStorage, PsqlMetaStorage> =
        Operon::new(service, storage.clone(), meta).with_options(operon_options);

    // This will start the Operon UI in the terminal,
    // where we can control the execution of the pipeline.
    operon_instance.run().await?;

    // After Operon has finished running,
    // we can retrieve the results from the storage.
    // As a side note, we could use `println!` here
    // since the UI is exited at this point.
    println!("{:?}", storage.get_intermediate([0, 0]).await?);
    println!("{:?}", storage.get_output([0, 0, 0]).await?);

    Ok(())
}
//# —————————————————————————— End of Example ————————————————————————— #//
