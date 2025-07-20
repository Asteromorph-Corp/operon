# Operon

A Rust-native workflow engine designed for parallel, incremental scheduling of [DAG-defined](#running-dag-defined-tasks) [multiplex](#multiplexing) tasks.
Powered by a PostgreSQL-based transactional backend, Operon specializes in orchestrating complex and long-running workflows with minimal downtime, flexible recovery, and high parallelism.

## Table of Contents

1. [Key Features](#key-features)
2. [Prerequisites](#prerequisites)
3. [Quick Start](#quick-start)
4. [Usage](#usage)
5. [Examples & Demo](#examples--demo)
6. [Roadmap](#roadmap)
7. [License](#license)

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

<!-- TODO: Fill the following sections using `ex1` -->
### Defining Entities

_Entities_ are typed values that are produced and consumed by tasks in Operon.
Any valid Rust type with a `PascalCase` name can be used as an entity, given that it implements the `Debug` and `Clone` traits.
For persistent database use, it is also recommended that the type implements `serde::Serialize` and `serde::Deserialize`.

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

```rust
// In examples/ex1/src/main.rs:

// This example pipeline is called "splitter", and it defines a simple flow:
// ┌─────────────────────────────────────────────────────────────┐
// │ Input  ——get_words——>  Intermediate  ——get_chars——>  Output │
// └─────────────────────────────────────────────────────────────┘
// where `Input`s are indexed by `[input_no]`,
// `Intermediate`s are indexed by `[input_no][word_no]`,
// and `Output`s are indexed by `[input_no][word_no][char_no]`.

operon::define_operon! {
    splitter = |Input<input_no>| {
        Intermediate<word_no> = get_words(Input) for input_no;
        Output<char_no> = get_chars(Intermediate) for input_no, word_no;
    }
}
```

Entities in Operon are paired with _named dimensions_ that represent the way you can iterate over the entities.
Simply put, these dimensions can be understood as _directions_ the entities repeat in.
For example, if you have an `Intermediate` entity that has two dimensions, `input_no` and `word_no`, you can think of it as a 2D grid where each cell is an `Intermediate` entity.

![Figure 1](docs/figures/figure1.svg)

The pipeline follows a few rules:

* The primary entity must be paired with exactly one dimension, the _primary dimension_.
* Each task must return one of the following two options:
  * A single entity.
  * A 1D vector of entities.
  In this case, the length of the returned vector will represent a new dimension that can be used to index the entities later in the pipeline.
* The dimension specifications must "make sense."
  <!-- TODO: Expand on what "make sense" means. -->

Invoking the `define_operon!` macro brings several utilities into scope:

* a `schema` module that contains the metadata of the pipeline;
* a `{PipelineName}Service` trait that provides the parsed tasks you would need to implement;
* a `{PipelineName}Storage` trait that exposes the storage interface for the entities;
* a `Psql{PipelineName}Storage` struct that serves as a default implementation of the storage interface using PostgreSQL;
* a helper `{pipeline_name}_handler()` function for launching the engine later.

Refer to the [`define_operon!` documentation](docs/define_operon_dsl.md) for more details on how to use the macro.

### Implementing the Service

### Implementing the Storage (Optional)

### Running Operon

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

## Examples & Demo

<!-- TODO: Provide a GIF/video demo of Operon in action. -->

You can find more examples in the [examples](examples/) directory of this repository.

## Roadmap

Operon is under active development. Planned features and improvements include:

* Having this README up to date with the latest changes.
* Adding documentation for the dimension system and the macro DSL.
* Updating the UI to scale better with larger workflows.
* Implementing alternative backends for the entity storage and the metadata storage.

Please reach out via [opening an issue](https://github.com/Asteromorph-Corp/operon/issues) if you have any suggestions or feature requests.

## License

Licensed under the GNU Affero General Public License, see [LICENSE](LICENSE) for details.
