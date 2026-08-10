# The define_operon! macro

`define_operon!` — declare a workflow pipeline with DAG-based task execution

## Synopsis

```rust
operon::define_operon! {
    pipeline_name = {
        [ TaskDecl ] ...
    }
}
```

where `TaskDecl` is:

```rust
[ #[operon( TaskAttr [, ...] )] ]
OutputEntity = task_function ( [ InputEntity ] ... ) for [ ( concurrency ) ] dimension [, ...] ;
```

where `TaskAttr` can be one of:

```rust
concurrency = concurrency
concurrency_env = ENV_VAR_NAME
ord = ( [ - ] dimension [, ...] )
```

where `ENV_VAR_NAME` is a bare identifier naming an environment variable, and each dimension in `ord` carries its own optional `-`.

where `OutputEntity` can be one of:

```rust
EntityIdent
EntityIdent < spawned_dimension >
```

where `InputEntity` can be one of:

```rust
EntityIdent
EntityIdent < dimension [, ...] >
```

and where `concurrency` is a positive integer literal, optional within the `for` clause.

Square brackets in this document indicate optional parts of the _documentation_, not the DSL.
Angle brackets and parentheses are literal tokens in the DSL.

## Description

`define_operon!` declares a workflow pipeline that defines the structure and dependencies of tasks operating on typed entities.
The macro generates the necessary traits, storage interfaces, and metadata structures required to execute the pipeline with the Operon runtime.

## Parameters

### Pipeline Name

The `pipeline_name` must be a valid Rust identifier in `snake_case`.
This name is used as the prefix for all generated types and traits:

- `{PipelineName}Service` trait
- `{PipelineName}Storage` trait
- `Psql{PipelineName}Storage` struct

The macro also emits a `schema` module, which is not prefixed.

### Task Declarations

Each task declaration specifies the transformation from input entities to output entities:

```text
OutputEntity = task_function ( InputEntity [, ...] ) for [ ( concurrency ) ] dimensional_context;
```

**`OutputEntity`**: Must be an `EntityIdent [ < spawned_dimension > ]`.

- `EntityIdent`: The identifier of the entity produced by this task.
  Should be in `PascalCase`.
- **`spawned_dimension`** (optional): When present, indicates this task produces multiple entities, creating a new dimension for iteration.
  If not present, the task produces an entity without spawning a new dimension.

**`task_function`**: A valid Rust identifier, which should be in `snake_case`, that becomes an async method in the generated service trait.

**`input_entities`**: Comma-separated list of entity references.
Each can be:

- `EntityIdent`: A single entity instance
- `EntityIdent < dimension [, ...] >`: A slice of entities across specified dimensions

**`dimensional_context`**: The `for` clause defines the set of indices over which the task runs.
A task's `for` clause must not repeat a dimension.

**`concurrency`** (optional): A positive integer which limits the number of concurrent jobs of that task within a single pipeline execution globally.
Each job corresponds to one index tuple in the task's context.
Additional jobs are queued.
If not specified, defaults to `1`.

### Task Attributes

A task declaration may be preceded by an `#[operon(...)]` attribute carrying any of the keys below.
Keys may be combined in one attribute, but each may appear only once per task.

**`concurrency = N`**: The same worker pool size as `for(N)`, written as a key.
A task may set its pool one way or the other, never both.

**`concurrency_env = ENV_VAR_NAME`**: Reads the pool size from the named environment variable, letting a deployment size the pool without a rebuild.
It sizes the pool in place of `for(N)` and `concurrency`, and a task may use only one of the three.
Every pool is resolved as the engine starts, before any job runs, and a variable that is unset, unparseable, or zero panics there.
A variable that is already set at macro-expansion time is validated then as well, turning a bad value into a compile error.

**`ord = (dimension [, ...])`**: The order in which the task's ready jobs are picked up, given as its dimensions in decreasing significance.
Each dimension is ascending by default, or descending when prefixed with `-`.
Only dimensions from the task's `for` clause may appear, at most once each.
Jobs that tie on every listed dimension run first-in-first-out, which is also the order used when no `ord` is given.

Ordering applies to jobs that are ready at the moment a worker frees up, and therefore the `ord` attribute is a bias rather than a guarantee.

```rust
operon::define_operon! {
    ranked = {
        Batch<batch_no> = fetch_batches();
        #[operon(concurrency_env = RANKED_WORKERS, ord = (-batch_no))]
        Report<report_no> = process(Batch) for batch_no;
    }
}
```

### Identifiers

Identifiers of entities, tasks, and dimensions are unique within the pipeline and cannot shadow each other.

## Dependency and Dimension System

Task declarations are **order-dependent**, and the resulting pipeline must form a **directed acyclic graph (DAG)** of entities and tasks.
This ensures that all dependencies can be resolved in a valid topological order before execution.

### Dependencies

Each task implicitly depends on the entities it consumes.
Tasks can only depend on entities that have been **previously declared**, so the dependency graph remains acyclic by design.

Standalone tasks are disallowed: every task must consume at least one entity.

### Dimensions

Dimensions define the **iteration context** for entities and tasks.
They act as indices over which tasks execute and entities are organized.
The rules below sketch the dimension system.

If you need further information or mathematical formalism, refer to [our technical report](https://arxiv.org/abs/2511.16080) for more details on the system.

#### Spawned dimensions

A task may declare a single `<spawned_dimension>` to represent multiple outputs (fan-out).
Each spawned dimension expands the iteration scope for downstream tasks.

#### The canonical dimension order

**The canonical dimension order** is the order in which the dimensions are introduced in the pipeline definition.

#### Dimensional context

Each task must specify the dimensions over which it runs in its `for` clause.
This context determines the dimension indices used for iteration.

Every dimension named in a task's `for` clause must be a dimension spawned by a _previous_ task.
This guarantees a topological order.

An output entity's dimensions are exactly the task's dimensional context dimensions plus the spawned dimension (if any).

#### Precedent and succedent dimensions

When a dimension is spawned, the spawned dimension **depends on** the task's dimensional context.

**Precedent dimensions** of `dim` are the complete set of dependencies leading into `dim`, that is, all dimensions from which a dependency path to `dim` exists.

Similarly, **succedent dimensions** of `dim` are the complete set of dependencies reachable from `dim`, that is, all dimensions from which a dependency path from `dim` exists.

#### Broadcasting

If an input entity's dimensions lack a dimension present in the task's context, the entity is treated as constant (broadcast) along that dimension.
When an input lacks a context dimension, the same value is supplied for all indices of that dimension within the task's iteration.

Broadcasting is logical only; no physical replication occurs in storage.

#### Slicing and aggregation

Input entities can be referenced across one or more of their own dimensions, and we define this behavior as **aggregating** (over a dimension):

- `Entity<dim, ...>` means a slice across given dimensions.
- `Entity` means a single entity bound to the current iteration context.

The slice is shaped in the declared `<dim1, ..., dimN>` order, which need not follow the canonical dimension order.

Input entities must not have any hanging dimensions: an entity's dimensions that do not serve as the context must be aggregated.
Also, if a dimension `dim` and its precedent dimension `dim_prec` both exist in an entity's dimensions, aggregating solely over `dim_prec` is disallowed.

Formally:

- `dims(slice) ⊆ dims(entity) ⊆ dims(context) ∪ dims(slice)`
- `∀dim ∈ dims(slice) : (Succ(dim) ∩ dims(entity)) ⊆ dims(slice)`, where `Succ(dim)` is the set of succedent dimensions of `dim`

Note that input entities **can** be aggregated by the context.
When a slice lists a dimension that's also in the context, the full axis is provided for each iteration point; the current index along that axis is ignored for that input.
It can be expensive, since a full `Vec` is constructed per iteration point.

## Examples

### Simple Linear Pipeline

```rust
operon::define_operon! {
    text_processor = {
        Document<doc_id> = fetch_documents();
        Sentence<sentence_id> = extract_sentences(Document) for doc_id;
        Word<word_id> = tokenize(Sentence) for doc_id, sentence_id;
        Token = analyze(Word) for doc_id, sentence_id, word_id;
    }
}
```

This creates a pipeline where:

- Each `Document` spawns multiple `Sentence` entities
- Each `Sentence` spawns multiple `Word` entities
- Each `Word` produces a single `Token`

### Fan-In Aggregation

```rust
operon::define_operon! {
    data_aggregator = {
        RawData<batch_id> = fetch_raw_data();
        ProcessedData<item_id> = process(RawData) for batch_id;
        Summary = aggregate(ProcessedData<item_id>) for batch_id;
    }
}
```

The `aggregate` task consumes all `ProcessedData` entities across the `item_id` dimension within each `batch_id` to produce a `Summary` per batch.

### Multi-Branch Pipeline

```rust
operon::define_operon! {
    ml_pipeline = {
        Dataset<dataset_id> = load_datasets();
        TrainSplit<fold_id> = create_folds(Dataset) for dataset_id;
        TestSplit<sample_id> = create_test_split(Dataset) for dataset_id;
        Model = train_model(TrainSplit<fold_id>) for dataset_id;
        Prediction<prediction_id> = predict(Model, TestSplit<sample_id>) for dataset_id;
        Metric = evaluate(Prediction<prediction_id>) for dataset_id;
    }
}
```

This demonstrates:

- Parallel task execution (`TrainSplit` and `TestSplit` can run concurrently)
- Tasks with multiple inputs (`predict` uses both `Model` and `TestSplit`)
- Dimension spawning and subsequent aggregation within each dataset

### Complex Pipeline Example

```rust
operon::define_operon! {
    cooking = {
        A<i> = alpha();
        B<j> = beta(A) for(8) i;
        C<k> = gamma(A) for(8) i;
        D    = delta(A, B<j>, C) for(4) i, j, k;
        E    = epsilon(B<j>, D<j>) for(4) i, k;
        F    = zeta(C<k>, E<k>) for i;
    }
}
```

This demonstrates:

- Worker pool sizing (`beta` and `gamma` tasks are executed using up to 8 dedicated workers, while `delta` and `epsilon` get 4 workers)
- Multi-dimensional iteration (task `delta` runs for each combination of `i`, `j`, `k`)
- Aggregation over the context (task `delta` consumes slices over `j` while iterating over `j`, meaning `delta` sees the full `Vec<B>` across `j` for each `(i, j, k)`)
- Dimension slicing (tasks consume slices like `B<j>`, a `Vec<B>` of all entities across the `j` dimension for each `(i, k)` context)

## Generated Artifacts

### Service Trait

The `{PipelineName}Service` trait contains async methods corresponding to each task:

```rust
#[async_trait::async_trait]
pub trait TextProcessorService: OperonService<
    JobEnum = schema::JobEnum,
    ResolutionEnum = schema::ResolutionEnum,
    TicketEnum = schema::TicketEnum,
> {
    // no spawn
    async fn analyze(&self, word: Word) -> Result<Token, Self::Error>;

    // spawns <sentence_id>
    async fn extract_sentences(&self, document: Document)
        -> Result<Vec<Sentence>, Self::Error>;
}
```

Implement it on a type that derives `OperonService`, which supplies the associated types above and the error type the task methods return.

#### Derive attributes

`#[derive(OperonService)]` reads an `#[operon(...)]` attribute on the same type, carrying any of the keys below.
Keys may be combined in one attribute, but each may appear only once.

**`error = MyError`**: The type the task methods report failure as, written as a bare type.
Defaults to `operon::error::UserError`, an alias of `Box<dyn std::error::Error + Send + Sync>`.

**`defined_at = "path"`**: The module `define_operon!` expanded in, written as a path inside a string.
The derive reads the generated `schema` module through it, so a service type declared outside that module needs it.
Defaults to `self`.

**`crate = "path"`**: The `operon` crate itself, written as a path inside a string.
Defaults to the name the dependency is declared under.

Note that `error` takes a type and the other two take strings.

Method signatures are determined by the task declaration's input and output specifications.
A parameter is named after the entity it carries, in `snake_case`, with the sliced dimensions appended: `Vec<B>` from `B<j>` arrives as `b_j`.

For an input declared as `Entity<dim1, dim2, ...>`, the generated parameter type is `Vec<...Vec<Entity>>` (N levels).
The slice is shaped in the declared slice-dimension order: `s[i1][i2]...[iN]` corresponds to `(dim1=i1, ..., dimN=iN)`.

For vector output, the vector order defines the spawned dimension's indices: each `output[i]` will be assigned an index `i`.

### Storage Trait and the Default Storage

The `{PipelineName}Storage` trait provides entity persistence methods.
It asks for a `get`/`put` pair per entity type:

```rust
#[async_trait::async_trait]
pub trait ExampleStorage: OperonStorage {
    async fn get_output_entity(&self, coordinate: [usize; N]) -> StorageResult<Option<OutputEntity>, Self::Error>;
    async fn put_output_entity(&self, entity: operon::Entity<N, OutputEntity>) -> StorageResult<(), Self::Error>;
    // ... methods for each entity type
}
```

where `N` is the number of dimensions of that entity and `operon::Entity<N, T>` is defined as the following:

```rust
pub struct Entity<const N: usize, T> {
    pub coordinate: [usize; N],
    pub value: T,
}
```

Coordinate parameters are passed in the canonical order of dimensions.

The trait additionally provides range accessors: one for each distinct slice the tasks consume, and one for each task that spawns a dimension.
For the [fan-in pipeline](#fan-in-aggregation) above, where `ProcessedData` is indexed by `[batch_id, item_id]`:

```rust
// for the input `ProcessedData<item_id>` of `aggregate`
async fn get_all_processed_data_item_id(&self, coordinate: [usize; 1]) -> StorageResult<Vec<ProcessedData>, Self::Error>;
// for the task `ProcessedData<item_id> = process(RawData) for batch_id`
async fn put_all_processed_data(&self, entity: operon::Entity<1, Vec<ProcessedData>>) -> StorageResult<(), Self::Error>;
```

A `get_all_*` takes the coordinate of the dimensions it does not iterate over, and its name appends the iterated dimensions in the declared slice order.
These default to walking the `get`/`put` pair one entity at a time, counting up from index `0` and stopping at the first coordinate that holds nothing.
Override them wherever the backend can serve a whole range in one query.
The values a `get_all_*` returns must be ordered by the dimensions it iterates over.

The pipeline-independent half of a storage backend is the `operon::OperonStorage` trait, which every implementation of the generated trait must also implement.
It declares the `Self::Error` these methods report failure as, prepares and clears the backend, and reads and writes the run footprint that lets a later run resume this one.
The three footprint methods default to no-ops, and a backend that leaves them alone has recovery disabled.

The `Psql{PipelineName}Storage` struct implementing both traits using the PostgreSQL backend will also be generated, serving as the default storage.

### Schema Module

A `schema` module containing metadata types:

```rust
pub mod schema {
    pub enum JobEnum {
        TaskFunction(Job<N>),
        // ... variants for each task
    }

    pub enum TicketEnum {
        TaskFunction(Ticket<N>),
        // ... variants for each task
    }

    pub enum ResolutionEnum {
        SpawnedDimension(Resolution<N>),
        // ... variants for each spawned dimension
    }
}
```

Variant names are the PascalCase form of the task or dimension they stand for.
A `Job` addresses one execution of a task by its coordinate, a `Ticket` tracks that execution's progress towards being runnable, and a `Resolution` records how far a spawned dimension extends under a given coordinate.
In each case `N` is the number of dimensions in the context of the task involved, and coordinates are ordered by the canonical dimension order.

The three enums are wired into the engine by `#[derive(OperonService)]`, so a pipeline never has to name them itself.

## Validation and Error Handling

The `define_operon!` macro performs validations at macro expansion time, especially checking the dependency and dimension rules.

The macro validates case conventions for identifiers and generates items using standardized casing:

- Pipelines: `snake_case` (e.g., `text_processor`)
- Entities: `PascalCase` (e.g., `Document`, `Token`)
- Tasks (functions): `snake_case` (e.g., `extract_sentences`)
- Dimensions: `snake_case` (e.g., `doc_id`, `word_id`)

Every entity type must be `Debug + Clone + serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static`.
The serde half of that is required even when the pipeline runs on a storage of your own, because `Psql{PipelineName}Storage` is generated either way.

### Common Compilation Errors

**Dependency rule violation**:

```rust
// ERROR: Undefined entity 'B'
A = create_a(B) for ...;
B = create_b(A) for ...;
```

**Dimension rule violation**:

```rust
// ERROR: Task create_c expected dimensions: i, j
// (`B` is left with a hanging `j`, which `create_c` neither iterates over nor aggregates)
B<j> = create_b(A) for i;
C = create_c(B) for i;
```

## Integration with Runtime

The generated artifacts integrate with the Operon runtime system:

```rust
use operon::options::{PsqlMetaStorageOptions, PsqlStorageOptions};
use operon::{Operon, OperonService};

// Implement the generated service trait
#[derive(OperonService)]
struct MyService;

#[async_trait::async_trait]
impl MyPipelineService for MyService {
    async fn my_task(&self, input: Input) -> Result<Vec<Output>, Self::Error> {
        // Task implementation
    }
}

// Build the generated storage (or implement your own) from its options
let storage = PsqlStorageOptions::new(&database_uri)
    .with_schema("data")
    .build::<PsqlMyPipelineStorage>()?;

// Build the metadata backend
let meta = PsqlMetaStorageOptions::new(&database_uri)
    .with_schema("meta")
    .build()?;

// Run the pipeline
let operon = Operon::new(MyService, storage, meta);
operon.run().await?;
```

`PsqlStorageOptions::build` needs to be told which storage it is building, either by a turbofish or by annotating the binding.
`Operon::new` takes the service and the storage by value or wrapped in an `Arc`.
Wrap them yourself to keep a handle for reading the results once the run is over.
Swapping `PsqlMetaStorageOptions` for `MemMetaStorageOptions` moves the metadata into memory, and together with an in-memory storage implementer, the pipeline runs without a database.

## Limitations

- Two or more dimensions cannot be spawned by a single task.
