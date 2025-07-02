use crate::configs::{DimensionId, EntityId};

pub type JobId = String;

/// An Operon job, which defines a transformation from one entity to another.
#[derive(Debug)]
pub struct JobConfig {
    /// Unique identifier for the job.
    pub id: JobId,
    /// Entities this job operates on.
    pub from: Vec<EntityId>,
    /// Entities this job produces.
    pub to: EntityId,
    /// Dimensions this job repeat on.
    pub dims: Vec<DimensionId>,
}
