use indexmap::IndexMap;

use crate::configs::{DimensionId, EntityId};

pub type JobId = String;

#[derive(Debug, Clone)]
pub struct JobArg {
    pub id: EntityId,
    pub over: Vec<DimensionId>,
}

/// An Operon job, which defines a transformation from one entity to another.
#[derive(Debug, Clone)]
pub struct JobConfig {
    /// Unique identifier for the job.
    pub id: JobId,
    /// Entities this job operates on.
    pub from: Vec<JobArg>,
    /// Entities this job produces.
    pub to: EntityId,
    /// Dimensions this job repeat on.
    pub dims: Vec<DimensionId>,
    /// Dimension this job spawns.
    pub spawn_dim: Option<DimensionId>,
}

pub type JobConfigMap = IndexMap<JobId, JobConfig>;
