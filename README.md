# Operon

A Rust-native workflow engine designed for parallel, incremental scheduling of [DAG-defined](#running-dag-defined-tasks) [multiplex](#multiplexing) tasks.
Powered by a PostgreSQL-based transactional backend, Operon specializes in orchestrating complex and long-running workflows with minimal downtime, flexible recovery, and high parallelism.

## Table of Contents

1. [Examples & Demo](#examples--demo)
2. [Key Features](#key-features)
3. [Prerequisites](#prerequisites)
4. [Quick Start](#quick-start)
5. [Usage](#usage)
    * [Installation](#installation)
    * [Defining Entities](#defining-entities)
    * [Defining the Pipeline](#defining-the-pipeline)
    * [Implementing the Service](#implementing-the-service)
    * [Implementing the Storage (Optional)](#implementing-the-storage-optional)
    * [Running Operon](#running-operon)
        * [UI Shell Commands](#ui-shell-commands)
6. [Roadmap](#roadmap)
7. [License](#license)

## Examples & Demo

![Demo 1](docs/figures/demo1.svg)

▲ Animation of running [ex2](examples/ex2) with Operon. _(100 primary entities, log level `Info`)_

<!-- [Demo 2](docs/figures/demo2.svg): This file was too large, maybe convert to another format? -->

▲ Animation of recovering from a poisoned run of ex2.

You can find more examples in the [examples](examples/) directory of this repository.

## Key Features

### Running DAG-Defined Tasks

Operon's primary use case is best described as _a known pipeline of an unknown number of tasks_.
It executes these tasks in parallel until all possible tasks have completed.

Here, a _task_ is a discrete unit of work that run in parallel, where each task's outputs ([_entities_](#defining-entities)) can serve as inputs for other tasks.
Tasks and their dependencies must be predefined, forming a directed acyclic graph (DAG).
This DAG's validity is checked at macro-expansion time.

### Multiplexing

Tasks in Operon are _multiplex_, meaning that one task may produce multiple entities of the same type (as a Rust `Vec`).
From another perspective, allowing multiplexing means that a task of a single type may be run multiple times, each using different input entities.
In this sense, a single node in the DAG represents a unique task _type_ that can be run repeatedly, where the number of individual tasks of that type cannot be known until upstream tasks produce the necessary entities.
Due to this, the number of tasks are quantified using an abstraction called [_named dimensions_](docs/dimension_system.md) instead of a simple count.

### Incremental Scheduling

Operon utilizes incremental scheduling, which means the scheduler never needs to know the entire task graph up front, saving memory and startup time.
As an event-driven system, each individual task runner is only aware of the tasks it can execute immediately, enabling efficient resource usage and pooling.

### Transactional Backend

Operon currently uses a PostgreSQL database as its backend for managing metadata.
The transactional design allows for atomic updates to task states, and by extension, reliable recovery from failures.

As a tradeoff, Operon often requires heavy database access, which may become a bottleneck for systems with high-throughput workloads.

### Interactive UI & Workflow Control

Operon provides a terminal-based TUI for real-time monitoring and control.
Users can track task progress, browse past logs, and interact with the workflow through shell-like commands — including pausing, resuming, or gracefully shutting down the engine.

### Per-Task Parallelism

Operon supports per-task parallelism, meaning that each task type maintains its own thread pool.
This is particularly useful for tasks that benefit from internal parallel execution or must adhere to external concurrency limits (e.g., database connections or API rate limits).

## Prerequisites

You will need the following to run Operon:

* [Rust](https://www.rust-lang.org/tools/install) (tested with Rust 1.88+)
* A working [PostgreSQL](https://www.postgresql.org/download/) database (version 14 or later)
  * A [connection URI](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING-URIS) that can access said database

We also recommend having [`tokio`](https://crates.io/crates/tokio) in your `Cargo.toml` dependencies.

## Quick Start

If you want to try out Operon, you can clone the repository and run the provided examples:

```bash
git clone https://github.com/Asteromorph-Corp/operon
cd operon/examples/ex1
# Make sure the URI points to a running PostgreSQL database.
POSTGRES_URI=<your_postgres_uri> cargo run --release
```

We recommend reading the source code of [ex1](examples/ex1/src/main.rs) to get a hang of how everything works.

## Usage

### Installation
<!-- [x] Operon is not published in crates.io, so we can only provide manual installation methods for now. -->

To use Operon in your Rust project, clone this repository:

```bash
git clone https://github.com/Asteromorph-Corp/operon
```

Once that's done, add the following to your project's `Cargo.toml`:

```toml
[dependencies]
# Assuming you cloned the repository to your home directory:
operon = { path = "~/operon/core" }
```

### Defining Entities

_Entities_ are typed values that are produced and consumed by tasks in Operon.
Any valid Rust type with a `PascalCase` name can be used as an entity, given that it implements the `Debug` and `Clone` traits.
For persistent database use, it is also recommended that the type implements `serde::Serialize` and `serde::Deserialize`.
An example of entity declarations is as follows:

```rust
// In examples/ex1/src/main.rs:

use operon::serde::{Serialize, Deserialize};

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
```

_Note 1_. These entity types must be directly accessible (without module scoping) in the scope where the [pipeline definition](#defining-the-pipeline) `define_operon!` macro is used.

_Note 2_. The engine can only recognize type names that are in `PascalCase` (a.k.a. `UpperCamelCase`) as defined in the [`heck` crate](https://docs.rs/heck/latest/heck).
Here are some examples of valid and invalid entity names:

| Invalid Name | Valid Name |
|--|--|
| `a` | `A` |
| `lowerCamel` | `UpperCamel` |
| `APIResponse` | `ApiResponse` |
| `XYCoordinates` | `XyCoordinates` / `XAndYCoordinates` |
| `_String` / `String_` | `StringEntity` |

### Defining the Pipeline

The _pipeline_ is the skeleton of the Operon workflow, defining _how_ the entities will be produced and consumed.
More specifically, the pipeline consists of the following components:

* **Name**: A unique identifier for the pipeline.
* **Primary entity type**: The type of the entity that will be used as the input to the pipeline.
* **Tasks**: A listing of tasks that will be executed in the pipeline.
Each task introduces a new type of entity to the pipeline, which can be used as input for subsequent tasks.

Additionally, entities in Operon are paired with _named dimensions_ that represent the way you can iterate over the entities.
Simply put, these dimensions can be understood as _directions_ the entities repeat in.
For example, if you have an `Intermediate` entity that has two dimensions, `input_no` and `word_no`, you can think of it as a 2D grid where each cell is an `Intermediate` entity.

![Figure 1](docs/figures/figure1.svg)

▲ An example of a 2D grid of `Intermediate` entities.

The following is an example of a pipeline definition using the `define_operon!` macro:

```rust
// In examples/ex1/src/main.rs:

operon::define_operon! {
    splitter = |Input<input_no>| {
        Intermediate<word_no> = get_words(Input) for input_no;
        Output<char_no> = get_chars(Intermediate) for input_no, word_no;
    }
}
```

![Figure 2](docs/figures/figure2.svg)

▲ A visual representation of the "splitter" pipeline.

Invoking the `define_operon!` macro brings several utilities into scope:

* a `schema` module that contains the metadata of the pipeline;
* a `{PipelineName}Service` trait that provides the parsed tasks [you would need to implement](#implementing-the-service);
* a `{PipelineName}Storage` trait that exposes [the storage interface](#implementing-the-storage-optional) for the entities;
* a `Psql{PipelineName}Storage` struct that serves as a default implementation of the storage interface using PostgreSQL;
* a helper `{pipeline_name}_handler()` function for [launching the engine](#running-operon) later.

The pipeline must follow a few rules that are enforced at macro-expansion time:

* The primary entity must be paired with exactly one dimension, the _primary dimension_.
* Each task takes a list of "arguments" or "inputs" that must be entities that were defined earlier in the pipeline.
  Each task input must be either a single entity (`EntityType`) or a slice across dimensions (`EntityType<dim1, dim2, ...>`).
* Each task must return one of the following two options:
  * A single entity, denoted `SpawnedEntityType`.
  * A 1D vector of entities, denoted `SpawnedEntityType<spawned_dimension_name>`.
    In this case, this task spawns a dimension that can be iterated over in subsequent tasks.
* The dimension specifications must be "well-formed," as thoroughly described in the [dimension system documentation](docs/dimension_system.md).
  * For illustration, take the list of `Intermediate`s as shown in [Figure 1](docs/figures/figure1.svg): `[["Good", "morning"], ["Bonjour"], ["Buenos", "días"]]`.
  * Writing `Intermediate<word_no>` represents a vector/slice of `Intermediate` entities indexed by `word_no`, which we will have for each `input_no` "coordinate."
    `["Good", "morning"]` or `["Bonjour"]` would be a valid example of such a vector.
  * However, writing `Intermediate<input_no>` would not be feasible.
  If we apply the same logic with above, we need a vector of `Intermediate` entities indexed by `input_no` "for each `word_no` coordinate."
  When `word_no` is `0`, we would have `["Good", "Bonjour", "Buenos"]`, but when `word_no` is `1`, what would we have — `["morning", ???, "días"]`?
  The range of `word_no` is unknown until the coordinate of `input_no` is fixed, so we cannot implicitly iterate over `word_no` while collapsing `input_no`.

We provide brief diagnostics for violations of these rules.
If you need further information, refer to the [`define_operon!` documentation](docs/define_operon_dsl.md) and the [dimension system documentation](docs/dimension_system.md) for more details on the system.

### Implementing the Service

The pipeline definition serves as a blueprint for the tasks that will be executed — now you would need to implement the actual logic of these tasks.
This is done by providing an `impl` for the `{PipelineName}Service` trait that was generated by the `define_operon!` macro.
Continuing with the previous example, you would implement the `splitter` pipeline as follows:

```rust
// In examples/ex1/src/main.rs (slightly modified):

use operon::{async_trait::async_trait, service::OperonService};
struct MySplitterService;
impl OperonService for MySplitterService {
    type JobEnum = schema::JobEnum;
    type ResolutionEnum = schema::ResolutionEnum;
}
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
        Ok(intermediate.0.chars().map(Output).collect())
    }
}
```

The exact signature of each task function is parsed from the pipeline definition, and will be provided in a docstring of the generated `{PipelineName}Service` trait.

### Implementing the Storage (Optional)

The Operon engine assumes all entities are accessible through a storage interface — we call this interface the `{PipelineName}Storage` trait.
We provide a struct `Psql{PipelineName}Storage` that already implements this trait using PostgreSQL, which can be constructed as follows:

```rust
// In examples/ex1/src/main.rs (slightly modified):

use operon::storage::StorageOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // ...
    let storage = PsqlSplitterStorage::new(
        StorageOptions::new("postgres://username:password@hostname:port/dbname")
            .with_schema(Some("data".to_string())),
    )?;
    // ...
}
```

You may also choose to implement your own storage interface by providing an `impl` for the `{PipelineName}Storage` trait.
Analogous to the service implementation, signatures of the functions you need to implement are parsed from the pipeline definition and will be provided in a docstring of the generated `{PipelineName}Storage` trait.

Having an alternative storage backend may be useful if you want to use a different database or have a quick in-memory storage for testing purposes.
However, note that the engine will not provide recoverability if the storage is volatile or you do not implement certain methods in the trait.

### Running Operon

Once you have all the pieces in place, you can run the Operon engine by constructing an `Operon` instance and calling the `.run()` method.
Note that the primary entity data must be initialized in the storage before running the engine.

```rust
// In examples/ex1/src/main.rs (slightly modified):

use operon::{
    operon::{Operon, OperonOptions},
    storage::{OperonStorage, StorageOptions},
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //# ——————————————————— Initializing Settings ————————————————————— #//
    let database_uri = "postgres://username:password@hostname:port/dbname";
    let service = Arc::new(MySplitterService);
    let storage = Arc::new(PsqlSplitterStorage::new(
        StorageOptions::new(&database_uri).with_schema(Some("ex1_data".to_string())),
    )?);
    let operon_options = OperonOptions::new(&database_uri)
        .with_meta_storage_schema(Some("ex1_meta".to_string()))
        .with_log_buffer_size(16384)
        .with_log_level(operon::log::Level::Info)
        .with_log_dump(Some("./logs".to_string()));

    //# —————————————————————— Initializing Data —————————————————————— #//
    // Initialize the storage with primary entity data.
    storage.init().await?;
    let num_inputs = 3;
    storage.put_input(0, "Good morning".to_string()).await?;
    storage.put_input(1, "Bonjour".to_string()).await?;
    storage.put_input(2, "Buenos días".to_string()).await?;

    //# ———————————————————————— Running Operon ——————————————————————— #//
    let operon_instance = Operon::new(service, storage, operon_options);
    operon_instance.run(splitter_handler(), num_inputs).await?;

    Ok(())
}
```

The `.run()` method will start the Operon engine that takes over the terminal and paints a UI during its execution.
If the execution is successful, the results will be stored in the storage, and you can retrieve them using the storage interface after the `.run().await?` call.

#### UI Shell Commands

The Operon engine provides a terminal-based text user interface (TUI) that allows you to interact with the engine and control the workflow.
The following commands are available in the UI:

```text
Navigation keys:
    ^C                  Clear input.
    ^D                  Exit.
    ^L                  Clear logs.
    ^Up, ^Down          Scroll logs 1 line.
    Up, Down            Scroll logs 5 lines.
    PgUp, PgDn          Scroll logs 20 lines.
    Esc                 Show most recent logs.

Commands:
    run [OPTIONS]       Start a new run using the best available restoration
                        (unless overridden by options).
        -f, --fresh         Start a fresh run, ignoring any existing data.
                            Takes precedence over `rebuild`.
        -r, --rebuild       Rebuild the run from trusted data before starting.
    check               Check the consistency of the data from the last run.
    exit                Exit the UI.
    clear               Clear the log buffer.
    quit [OPTIONS]      Stop all jobs and exit the UI. Defaults to graceful shutdown.
        -f, --force         Force quit.
        -n, --no-exit       Don't exit the UI.
    pause [OPTIONS] [<JOB_TYPE>[ ...]]
                        Pause executing new jobs.
        -c, --cascade       Cascade the pause command to dependent jobs.
    resume [<JOB_TYPE>[ ...]]
                        Resume paused jobs.
    help                Print this help message.
```

## Roadmap

Operon is under active development. Planned features and improvements include:

* Implementing the following features.
  These should inherently follow from the current model, but are not yet implemented due to technical difficulties:
  * Support for zero-dimension entities or tasks ("singletons" or "scalars").
  * Support for jobs with no dependencies that create "source" entities other than the primary entity.
* Adding documentation for the dimension system and the macro DSL.
* Updating the UI to scale better with larger workflows.
* Adding support for running the engine without a UI, potentially outside a binary-executable context.
* Implementing alternative backends for the entity storage and the metadata storage.

Please reach out via [opening an issue](https://github.com/Asteromorph-Corp/operon/issues) if you have any suggestions or feature requests.

## License

Licensed under the GNU Affero General Public License, see [LICENSE](LICENSE) for details.
