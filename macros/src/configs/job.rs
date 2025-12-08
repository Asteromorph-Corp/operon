use indexmap::IndexMap;

pub type JobId = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JobArg {
    pub id: syn::Ident,
    pub over: Vec<syn::Ident>,
}

/// An Operon job, which defines a transformation from one entity to another.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JobConfig {
    /// Unique identifier for the job.
    pub id: JobId,
    /// Entities this job operates on.
    pub from: Vec<JobArg>,
    /// Entities this job produces.
    pub to: syn::Ident,
    /// Dimensions this job repeat on.
    pub dims: Vec<syn::Ident>,
    /// Dimension this job spawns.
    pub spawn_dim: Option<syn::Ident>,
    /// The pool size for this job.
    pub pool_size: usize,
}

pub type JobConfigMap = IndexMap<JobId, JobConfig>;
