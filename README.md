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

Here, a _task_ is a discrete unit of work that run in parallel, where each task's outputs (_entities_) can serve as inputs for other tasks.
Tasks and their dependencies must be predefined, forming a directed acyclic graph (DAG).
This DAG's validity is checked at macro-expansion time.

### Multiplexing

Tasks in Operon are _multiplex_, meaning that one task may produce multiple entities of the same type (as a Rust `Vec`).
From another perspective, allowing multiplexing means that a task of a single type may be run multiple times, each using different input entities.
In this sense, a single node in the DAG represents a unique task _type_ that can be run repeatedly, where the number of individual tasks of that type cannot be known until upstream tasks produce the necessary entities.
Due to this, the number of tasks are quantified using an abstraction called [_dimensions_](#dimensions-and-multiplexing) instead of a simple count.

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

* [Rust](https://www.rust-lang.org/tools/install) (tested with Rust 1.75+)
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

<!-- TODO: Provide instructions for installing Operon. -->

### Configuration
<!-- FIXME: Rewrite this whole section, this information is out of date. -->

In your crate root (`src/main.rs` or `src/lib.rs`), you can configure Operon using the `operon_macros::include_operon!` macro.
An example configuration might look like this (from the [ex1](examples/ex1/src/lib.rs) example):

```rust
operon_macros::include_operon! {
    config = {
        // storage.data: configurations for the default entity storage impelementation
        storage.data.uri = "postgres://user:password@hostname:port/operon-db",
        storage.data.pool_size = 16,
        storage.data.schema = "data",
        // storage.metadata: configurations for the metadata backend
        storage.metadata.uri = "postgres://user:password@hostname:port/operon-db",
        storage.metadata.schema = "metadata",
        // log: configurations for the logging system
        log.level = "debug",
        log.buffer_size = 1024,
        log.dump = true,
        log.dump_path = "logs",
    };
    types = {
        // The type definitions of entities go here.
    };
    // Name of the default entity storage implementation, if any.
    use_psql_storage = DataStorage;
}
```

Below is a reference table of the configuration options (values marked with `*` are required):

| Key | Value Type | Description |
|:--|:--|:--|
| `storage.data.uri` | `String` | The PostgreSQL [connection URI](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING-URIS) for the default entity storage implementation. <br> Only used when `use_psql_storage` is set. |
| `storage.data.pool_size` | `usize` | The size of the connection pool for the default entity storage implementation. (Default: 16) <br> Only used when `use_psql_storage` is set. |
| `storage.data.schema` | `String` | The schema name for the default entity storage implementation. Uses the default schema if not specified. <br> Only used when `use_psql_storage` is set. |
| `storage.metadata.uri` | `String`* | The PostgreSQL [connection URI](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING-URIS) for the metadata backend. |
| `storage.metadata.pool_size` | `usize` | The size of the connection pool for the metadata backend. (Default: 16) |
| `storage.metadata.schema` | `String` | The schema name for the metadata backend. Uses the default schema if not specified. |
| `log.level` | `String` | The minimum logging level the UI will show. <br> (Default: `info`, possible values: `trace`, `debug`, `info`, `warn`, `error`) |
| `log.buffer_size` | `usize` | The size of the UI log buffer. (Default: 1024) |
| `log.dump` | `bool` | Whether to dump all logs to files. (Default: `false`) |
| `log.dump_path` | `String` | The path to the directory where logs will be dumped. (Default: `logs`) |

### Defining Entities and Tasks
<!-- FIXME: Rewrite this whole section, this information is out of date. -->

In the `types = {};` block of the `include_operon!` macro, you can define the entities and tasks that Operon will manage.
An example definition set might look like this (from the [ex1](examples/ex1/src/lib.rs) example):

```rust
types = {
    #[entity(primary, dims = ["i"])]
    pub struct A(pub String);

    #[entity(dims = ["i", "j"], def = "beta|i", from = ["A"], pool = 8)]
    pub struct B(pub A, pub usize);

    #[entity(dims = ["i", "k"], def = "gamma|i", from = ["A"], pool = 8)]
    pub struct C(pub usize);

    #[entity(
        dims = ["i", "j", "k"],
        def = "delta | i, j, k",
        from = ["A", "B", "C"],
        pool = 4
    )]
    pub struct D {
        pub a: A,
        pub b: B,
        pub c: C,
    }

    #[entity(dims = ["i", "k"], def = "epsilon | i, k", from = ["B | j", "D|j"], pool = 4)]
    pub struct E {
        pub b: Vec<B>,
        pub d: Vec<D>,
    }

    #[entity(dims = ["i"], def = "zeta|i", from = ["C|k", "E|k"])]
    pub enum F {
        Success {
            c: Vec<C>,
            e: Vec<E>,
        },
        Failure(String, Option<C>, Option<E>),
    }
};
```

Each entity is a `struct` or an `enum` defined with a set of attributes:

| Attribute | Description |
|:--|:--|
| `primary` | Marks the entity as a primary entity — the source data. |
| `dims` | Specifies the dimensions of the entity. |
| `def` | The task function that defines this entity. |
| `from` | The input entities that `def` depends on. (_The parameters of the_ `def` _function_.) |
| `pool` | The number of parallel instances of `def` that can run concurrently. <br> If unspecified, defaults to 1. |

Once the entities and tasks are defined and adheres to the [rules](#entity-definition-rules), the tasks will be available for implementation in the `operon::OperonService` trait.
In the example above, the `def` functions for each entity would be parsed as follows:

```rust
use anyhow::Result;
use async_trait::async_trait;
use operon::entity::*;
#[async_trait]
pub trait OperonService {
    async fn beta(&self, a: &A) -> Result<Vec<B>>;
    async fn gamma(&self, a: &A) -> Result<Vec<C>>;
    async fn delta(&self, a: &A, b: &B, c: &C) -> Result<D>;
    async fn epsilon(&self, b_j: &[B], d_j: &[D]) -> Result<E>;
    async fn zeta(&self, c_k: &[C], e_k: &[E]) -> Result<F>;
}
```

Please consult the internal documentation of `OperonService` for more details on how to implement these functions.

<!-- FIXME: The following two sections are overly complex and should be simplified, rewritten, or moved to a separate document. While these are essential details, they are overwhelming for a first-time user. -->
#### Dimensions and Multiplexing

In the above example, the `dims` attribute specifies the dimensions of each entity.
These dimensions are also used to define the multiplexing behaviour of tasks and entities — for example, in the `E` entity's attributes, the `def` "`epsilon`" refers to the dimensions `i` and `k`, while `from` names entities `B` and `D`, who refer to the dimension `j`.

Dimensions always take a range that looks like `[0, U)`, where `U` is the upper bound of that dimension.
`U` is determined by the output of the task that _spawned_ this dimension (i.e. the task that first introduced this dimension).
Whenever an entity or task refers to a dimension, it means that it _repeats_ on that dimension:

* `A` having `dims = ["i"]` means that `A` repeats on a single dimension `i`, so there are `U(i)` entities of type `A`.
* `D` having `dims = ["i", "j", "k"]` means that `D` repeats on three dimensions: `i`, `j`, and `k`, so there will be as many entities of type `D` as there are combinations of "coordinates" in the three dimensions.
* `beta|i` means that the `beta` task will run for all values of `i` in the range `[0, U(i))`.
* `D|j` as a parameter of `epsilon|i, k` means that the `epsilon` task will be run for all values of `i` and `k`, and that `D` will be collected over the dimension `j` (i.e. all entities of type `D` with the same `i` and `k` will be collected into a single `Vec<D>`).

Note that some dimensions depend on other dimensions, like `j` in the example above.
This is because the task that spawns the dimension `j` is `beta`, which itself repeats on the dimension `i`.
For different values of `i`, the result of `beta` may yield different values of `U(j)`, which is why `j` is _dependent_ on `i`.
When an entity has multiple dimensions, the dimensions are always ordered by their dependencies, so if we were to represent the `D` entities as a `Vec<Vec<Vec<D>>>` or a `&[[[D]]]`, it would be indexed as `D[i][j][k]`.

<!-- TODO: Separate some of the following to a document file -->
#### Entity Definition Rules

There are several rules that the above definition must follow for Operon to work correctly:

* Each entity, dimension and task must have a unique name.
* Definitions are order-sensitive.
* All entities must be a `struct` or an `enum`.
  * The entities must be `#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]`-able.
  * All entities and their fields must be `pub`.
  * You may use previously defined entities when defining other entities.
  * Simple type aliases (like `pub type A = String;`) are not allowed; using a tuple struct (like `pub struct A(pub String);`) is recommended.
* The first entity must have the `primary` attribute, and other entities must not have it.
  * If you don't need a primary entity that acts as the source data, use an empty struct with the `primary` attribute.
* All entities must have the `dims` attribute.
  * The primary entity must have exactly one dimension, which we call the "primary dimension", representing how many times the whole workflow will run.
  Again, if you want to run the workflow only once, have the primary dimension's upper bound be 1.
  * Each entity may introduce _up to one_ new dimension, other dimensions must be already defined beforehand.
* All entities _except_ the primary entity must have the `def` attribute.
  * The `def` attribute must be a _dimension-annotated function name_: a combination of a valid Rust function name and a list of dimensions, separated by a pipe (`|`).
  * Dimensions listed in the `def` attribute must be precisely the dimensions in `dims` except the dimension this entity introduces.
  * If this entity introduces a new dimension, `def` returns a `Vec` of the entity type, otherwise it returns a single instance of the entity type.
* All entities with the `def` attribute must have the `from` attribute.
  * The `from` attribute must be a list of dimension-annotated entity names.
  * Entities and dimensions listed in the `from` attribute must have been defined strictly before this entity.
  * For each entity listed in `from`, taking the union of the repeating-on dimensions and the task's `def` dimensions must yield a superset of the entity's `dims`.
  For example, in `epsilon | i, k` whose `from` is `["B | j", "D|j"]`, `["i", "k"] U ["j"]` is a superset of both `["i", "j"]` (for `B`) and `["i", "j", "k"]` (for `D`), so this is a valid definition.
  * If the associated entity is supposed to be a source without dependencies, use `[]` as the `from` value.

### Running Operon
<!-- FIXME: Rewrite this whole section, this information is out of date. -->

Once you have configured Operon via the `include_operon!` macro, you can run the Operon engine using the `operon::Operon` struct's `run` method.
Operon is meant to be run as a binary application, so you will have a `main` function that initializes the engine and starts it most of the time.

_Warning_: Using `println!` or other routines that write to `stdout` or `stderr` after the Operon engine has started will cause the UI to malfunction.
Use the `log` crate or the logging macros provided by Operon to log messages instead.

A typical usage might look like this:

```rust
// src/main.rs
operon_macros::include_operon! {
    // ... configuration and types as shown above ...
    use_psql_storage = DataStorage;
}
struct MyService;
impl operon::OperonService for MyService {
    // ... implement each task function ...
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let primary_ub = 100; // The upper bound of the primary dimension.
    let storage = DataStorage::new().await?;
    // Insert primary entities into the storage.
    for i in 0..primary_ub {
        let a = A(format!("Primary entity {}", i));
        storage.put_a(i, &a).await?;
    }
    let service = MyService;
    let operon = operon::Operon::new(storage, service);
    operon.run(primary_ub).await?;
    Ok(())
}
```

Note that the `run` method takes a single argument, which is the upper bound of the primary dimension.

Also, by the time you call `run`, all primary entities must already be present in the entity storage.
You can use the `put_*` methods of the storage to insert primary entities, as shown in the example above.

Finally, you can give a custom `OperonStorage` implementation without the `use_psql_storage` configuration if you want to use a different storage backend or perhaps an in-memory storage for testing purposes.
Refer to the generated documentation of `OperonStorage` for more details.

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
