use crate::configs::{DimensionConfigMap, DimensionId, EntityConfigMap, EntityId, JobConfigMap};

pub struct AllConfig {
    pub primary_dimension: DimensionId,
    pub primary_entity: EntityId,
    pub dimensions: DimensionConfigMap,
    pub entities: EntityConfigMap,
    pub jobs: JobConfigMap,
}
