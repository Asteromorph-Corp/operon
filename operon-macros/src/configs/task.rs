use indexmap::IndexMap;

/// Ordering direction for an `ord=` priority dimension (macro-internal mirror of
/// `operon::Direction`; no value-level import of the runtime crate required).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Ascending,
    Descending,
}

/// The concurrency (pool size) specification for a task.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PoolSizeSpec {
    /// A fixed concurrency level, e.g. from `for(8)` or `#[operon(concurrency=8)]`.
    Literal(usize),
    /// Read from a named environment variable at runtime, e.g. `#[operon(concurrency_env=VAR)]`.
    Env(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskArg {
    pub id: syn::Ident,
    pub over: Vec<syn::Ident>,
}

/// An Operon task, which defines a transformation from one entity to another.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskConfig {
    /// Unique identifier for the task.
    pub id: syn::Ident,
    /// Entities this task operates on.
    pub from: Vec<TaskArg>,
    /// Entities this task produces.
    pub to: syn::Ident,
    /// Dimensions this task repeat on.
    pub dims: Vec<syn::Ident>,
    /// Dimension this task spawns.
    pub spawn_dim: Option<syn::Ident>,
    /// The pool size specification for this task.
    pub pool_size: PoolSizeSpec,
    /// Priority ordering for the task queue. Empty means FIFO.
    pub priority: Vec<(syn::Ident, Direction)>,
}

pub type TaskConfigMap = IndexMap<syn::Ident, TaskConfig>;
