// # Operon Example 1: Splitter
//
// This example code serves as a basic introduction to Operon.
// Here, we demonstrate a simple use case of Operon,
// where we split a string into words and then into characters.
//
// To run this example as-is, you will need an URI to a working PostgreSQL database,
// and use the following shell command in this example's root directory (`operon/examples/ex1`):
// `POSTGRES_URI=<database_uri> cargo run --release`
//
// This example is meant to be run as a binary,
// and was tested with Rust 1.90.0-nightly and PostgreSQL 17.5.

use std::sync::Arc;

use operon::async_trait::async_trait;
use operon::define_operon;
use operon::operon::{Operon, OperonOptions};
use operon::serde::{Deserialize, Serialize};
use operon::service::OperonService;
use operon::storage::{OperonStorage, StorageOptions};

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
// The following attribute can be omitted
// if you use `serde::{Serialize, Deserialize}`
// instead of `operon::serde::{Serialize, Deserialize}`.
#[serde(crate = "operon::serde")]
struct Intermediate(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "operon::serde")]
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
    splitter = |Input<input_no>| {
        Intermediate<word_no> = get_words(Input) for input_no;
        Output<char_no> = get_chars(Intermediate) for input_no, word_no;
    }
}
//# ——————————————————————————————————————————————————————————————————— #//

//# ———————————————————— C. Service Implementation ———————————————————— #//
// Define the behaviour of each step in the pipeline.

// The necessary functions are generated as traits,
// so we will need to create a struct to implement them.
// The struct must implement `OperonService`.
#[derive(OperonService)]
struct MySplitterService;

// The following is the main trait that we need to implement.
// Note that its name depends on the pipeline name we gave above.
// Consult the generated documentation for this trait
// to see what methods are required.
#[async_trait]
impl SplitterService for MySplitterService {
    async fn get_words(
        &self,
        input: Input,
    ) -> Result<Vec<Intermediate>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(input
            .split_whitespace()
            .map(|s| Intermediate(s.to_string()))
            .collect())
    }

    async fn get_chars(
        &self,
        intermediate: Intermediate,
    ) -> Result<Vec<Output>, Box<dyn std::error::Error + Send + Sync>> {
        // To print something to the UI,
        // we can use the `log` crate directly,
        // or use the provided `operon::log` module.
        // Do not write to `stdout` or `stderr` directly,
        // as it will interfere with the Operon UI.
        operon::log::info!("Processing intermediate: {}", intermediate.0);
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

    //# ——————————————————— Initializing Settings ————————————————————— #//
    // The service is what we defined above —
    // a struct that holds the logic for the pipeline.
    let service = Arc::new(MySplitterService);

    // The storage is where all the data is stored and retrieved.
    // Here, we use `PsqlSplitterStorage` for the storage.
    // This is an automatically generated storage implementation
    // based on a PostgreSQL database.
    let storage = Arc::new(PsqlSplitterStorage::new(
        StorageOptions::new(&database_uri).with_schema(Some("ex1_data".to_string())),
    )?);

    // The `OperonOptions` struct is used to configure the Operon instance.
    // The URI to a PostgreSQL database (which will hold the metadata for Operon) is REQUIRED here.
    // Note that we used the same URI as the data storage,
    // but we may use a different one as long as it points to a valid PostgreSQL database.
    // We strongly suggest using different schema names if you use the same database.
    // We can also set some additional options related to the general performance of Operon here.
    let operon_options = OperonOptions::new(&database_uri)
        .with_meta_storage_schema(Some("ex1_meta".to_string()))
        .with_log_buffer_size(16384) // How many log messages should the UI remember?
        .with_log_level(operon::log::Level::Info) // What should be the minimum log level to show in the UI?
        .with_log_dump(Some("./logs".to_string())); // Optionally, which directory should all logs be dumped to?

    //# —————————————————————— Initializing Data —————————————————————— #//
    // It is good practice to initialize the storage before touching the data.
    // NOTE THAT if there is a predefined table in the database
    // that matches the name but not the exact dimensions defined in the macro,
    // the initialization will not overwrite it, resulting in an error.
    // This happens when you shift around the definitions in the macro
    // after you had already run the pipeline once or more.
    // In this case, you might want to drop the related tables manually
    // (i.e., run `DROP SCHEMA ex1_data CASCADE;` in the database),
    // or point to a different, fresh schema in `StorageOptions`.
    storage.init().await?;

    // The primary entities (`Input`s in this case) MUST be in the storage before running Operon.
    // It is possible to prepare the data externally
    // as long as the storage is persistent and the data is in the expected format,
    // but if unsure, we recommend using the built-in `put_*` methods as shown below.
    let num_inputs = 3;
    storage.put_input(0, "Hello World".to_string()).await?;
    storage.put_input(1, "Hello Operon".to_string()).await?;
    storage.put_input(2, "".to_string()).await?;

    //# ———————————————————————— Running Operon ——————————————————————— #//
    // Now we finally run Operon.

    // Initialize an Operon struct with the configurations above.
    // We cloned the `storage` Arc because we intend to access the storage
    // after the Operon instance has finished running,
    // but this is entirely optional.
    let operon_instance = Operon::new(service, storage.clone(), operon_options);

    // To run the Operon instance,
    // we need to provide a handler for the pipeline,
    // and the number of primary entities that Operon will process.
    // A function that provides a handler for the pipeline
    // is automatically generated by the `define_operon!` macro.
    // This will start the Operon UI in the terminal,
    // where we can control the execution of the pipeline.
    operon_instance.run(splitter_handler(), num_inputs).await?;

    // After Operon has finished running,
    // we can retrieve the results from the storage.
    // As a side note, we could use `println!` here
    // since the UI is exited at this point.
    println!("{:?}", storage.get_intermediate(0, 0).await?);
    println!("{:?}", storage.get_output(0, 0, 0).await?);

    Ok(())
}
//# —————————————————————————— End of Example ————————————————————————— #//
