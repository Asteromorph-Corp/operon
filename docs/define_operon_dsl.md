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
OutputEntity = task_function ( [ InputEntity ] ... ) for [ ( concurrency ) ] dimension [, ...] ;
```

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

and where `concurrency` is an optional positive integer literal.

Square brackets in this document indicate optional parts of the _documentation_, not the DSL. Angle brackets and parentheses are literal tokens in the DSL.

## Description

`define_operon!` declares a workflow pipeline that defines the structure and dependencies of tasks operating on typed entities. The macro generates the necessary traits, storage interfaces, and metadata structures required to execute the pipeline with the Operon runtime.

## Parameters

### Pipeline Name

The `pipeline_name` must be a valid Rust identifier in `snake_case`. This name is used as the prefix for all generated types and traits:

- `{PipelineName}Service` trait
- `{PipelineName}Storage` trait
- `Psql{PipelineName}Storage` struct
- `{pipeline_name}_handler()` function

### Task Declarations

Each task declaration specifies the transformation from input entities to output entities:

```text
OutputEntity = task_function ( InputEntity [, ...] ) for [ ( concurrency ) ] dimensional_context;
```

**`OutputEntity`**: Must be an `EntityIdent [ < spawned_dimension > ]`.

- `EntityIdent`: The identifier of the entity produced by this task. Should be in `PascalCase`.
- **`spawned_dimension`** (optional): When present, indicates this task produces multiple entities, creating a new dimension for iteration. If not present, the task produces an entity without spawning a new dimension.

**`task_function`**: A valid Rust identifier should be in `snake_case` that will become an async method in the generated service trait.

**`input_entities`**: Comma-separated list of entity references. Each can be:

- `EntityIdent`: A single entity instance
- `EntityIdent < dimension [, ...] >`: A slice of entities across specified dimensions

**`dimensional_context`**: The `for` clause defines the set of indices over which the task runs. A task’s `for` clause must not repeat a dimension.

**`concurrency`** (optional): A positive integer which limits the number of concurrent jobs for that task type within a single pipeline execution globally. Each job corresponds to one index tuple in the task’s context. Additional jobs are queued. If not specified, defaults to `1`.

### Identifiers

Identifiers of entities, tasks, and dimensions are unique within the pipeline, cannot shadow each other.

## Dependency and Dimension System

Task declarations are **order-dependent**, and the resulting pipeline must form a **directed acyclic graph (DAG)** of entities and tasks. This ensures that all dependencies can be resolved in a valid topological order before execution.

### Dependencies

Each task implicitly depends on the entities it consumes. Tasks can only depend on entities that have been **previously declared**, thereby the dependency graph remains acyclic by design.

Standalone jobs are disallowed: every task must consume at least one entity.

### Dimensions

Dimensions define the **iteration context** for entities and tasks. They act as indices over which tasks execute and entities are organized. Below illustrates brief rules of the dimension system.

If you need further information or mathematical formalism, refer to the [[dimension system documentation]] for more details on the system.

#### Spawned dimensions

A task may declare a single `<spawned_dimension>` to represent multiple outputs (fan-out). Each spawned dimension expands the iteration scope for downstream tasks.

#### The canonical dimension order

**The canonical dimension order** is defined as an order they are introduced in the pipeline definition.

#### Dimensional context

Each task must specify the dimensions over which it runs in it's `for` clause. This context determines the dimension indices used for iteration.

Every dimension named in a task’s `for` clause must be either a dimension spawned by a _previous_ task. This guarantees a topological order.

An output entity’s dimensions are exactly the task’s dimensional context dimensions plus the spawned dimension (if any).

#### Precedent and succedent dimensions

When a dimension is spawned, the spawned dimension **depends on** the task's dimensional context.

**Precedent dimensions** of `dim` are the complete set of dependencies leading into `dim`, that is, all dimensions from which a dependency path to `dim` exists.

Similarly, **succedent dimensions** of `dim` are the complete set of dependencies reachable from `dim`, that is, all dimensions from which a dependency path from `dim` exists.

#### Broadcasting

If dimensions of an input entity lacks a dimension present in the task’s context, the entity is treated as constant (broadcast) along that dimension. When an input lacks a context dimension, the same value is supplied for all indices of that dimension within the task’s iteration.

Broadcasting is logical only; no physical replication occurs in storage.

#### Slicing and aggregation

Input entities can be referenced across one or more dimensions (within its dimensions), and we define this behavior as **aggregating** (over a dimension):

- `Entity<dim, ...>` means a slice across given dimensions.
- `Entity` means a single entity bound to the current iteration context.

The slice is shaped in the declared `<dim1, ..., dimN>` order. (need not follow the canonical dimension order)

Input entities must not have any hanging dimensions: an entity's dimensions that do not serve as the context must be aggregated. Also, if a dimension `dim` and its precedent dimension `dim_prec` both exist in an entity's dimensions, aggregating solely over `dim_prec` is disallowed.

Formally:

- `dims(slice) ⊆ dims(entity) ⊆ dims(context) ∪ dims(slice)`
- `∀dim ∈ dims(slice) : (Succ(dim) ∩ dims(entity)) ⊆ dims(slice)`, where `Succ(dim)` is the set of succedent dimensions of `dim`

Note that input entities **can** be aggregated by the context. When a slice lists a dimension that’s also in the context, the full axis is provided for each iteration point; the current index along that axis is ignored for that input. It can be expensive, since a full `Vec` is constructed per iteration point.

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
- Dimension slicing (tasks consume slices like `B<j>` (`Vec<B>` of all entities across `j` dimension, for each `(i,k)` context)

## Generated Artifacts

### Service Trait

The `{PipelineName}Service` trait contains async methods corresponding to each task:

```rust
type StdError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait::async_trait]
pub trait TextProcessorService: OperonService {
    // no spawn
    async fn analyze(&self, word: Word) -> Result<Token, StdError>;

    // spawns <sentence_id>
    async fn extract_sentences(&self, doc: Document)
        -> Result<Vec<Sentence>, StdError>;
}
```

Method signatures are determined by the task declaration's input and output specifications.

For an input declared as `Entity<dim1, dim2, ...>`, the generated parameter type is `Vec<...Vec<Entity>>` (N levels). The slice is shaped in the declared slice-dimension order: `s[i1][i2]...[iN]` corresponds to `(dim1=i1, ..., dimN=iN)`.

For vector output, the vector order defines the spawned dimension's indices: each `output[i]` will be assigned an index `i`.

### Storage Trait and the Default Storage

The `{PipelineName}Storage` trait provides entity persistence methods:

```rust
#[async_trait::async_trait]
pub trait ExampleStorage: OperonStorage {
    async fn get_output_entity(&self, coordinate: [usize; N]) -> Result<Option<OutputEntity>, Self::Error>;
    async fn put_output_entity(&self, entity: operon::Entity<N, OutputEntity>) -> Result<(), Self::Error>;
    // ... methods for each entity type
}
```

where `N` is a constant representing the total number of dimensions in the pipeline and `operon::Entity<N, T>` is defined as the following:

```rust
pub struct Entity<const N: usize, T> {
    pub coordinate: [usize; N],
    pub value: T,
}
```

Coordinate parameters are passed in the canonical order of dimensions.

The `Psql{PipelineName}Storage` struct implementing the `{PipelineName}Storage` using the PostgreSQL backend will also be generated, serving as the default storage.

### Schema Module

A `schema` module containing metadata types:

```rust
pub mod schema {
    pub enum JobEnum {
        TaskFunction { /* dimensions */ },
        // ... variants for each task
    }

    pub enum ResolutionEnum {
        OutputEntity(DimensionTuple),
        // ... variants for each entity type
    }
}
```

where `DimensionTuple` is `(usize, ...)` ordered by the canonical dimension order.

## Validation and Error Handling

The `define_operon!` macro performs validations at macro expansion time, especially checking the dependency and dimension rules.

The macro validates case conventions for identifiers and generates items using standardized casing:

- Pipelines: `snake_case` (e.g., `text_processor`)
- Entities: `PascalCase` (e.g., `Document`, `Token`)
- Tasks (functions): `snake_case` (e.g., `extract_sentences`)
- Dimensions: `snake_case` (e.g., `doc_id`, `word_id`)

Every entity type must be `Debug + Clone`. Additional `serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static` to use the default PostgreSQL storage.

### Common Compilation Errors

**Dependency rule violation**:

```rust
// ERROR: entity 'B' referenced before it is defined
A = create_a(B) for ...;
B = create_b(A) for ...;
```

**Dimension rule violation**:

```rust
// ERROR: in task `create_C`: entity 'B' has hanging dimensions ['j']
B<j> = create_b(A) for i;
C = create_c(B) for i;
```

## Integration with Runtime

The generated artifacts integrate with the Operon runtime system:

```rust
use operon::options::{OperonOptions, PsqlMetaStorageOptions, PsqlStorageOptions};
use operon::Operon;

// Implement the generated service trait
struct MyService;
impl MyPipelineService for MyService {
    async fn my_task(&self, input: Input) -> Result<Vec<Output>, _> {
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
let operon = Operon::new(MyService, storage, meta).with_options(OperonOptions::new());
operon.run().await?;
```

## Limitations

- Two or more dimensions cannot be spawned by a single task.
