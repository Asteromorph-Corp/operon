use crate::configs::{DimensionConfigMap, DimensionId, EntityConfigMap, EntityId, JobConfigMap};

#[derive(Debug)]
pub struct AllConfig {
    pub service_id: String,
    pub primary_dimension: DimensionId,
    pub primary_entity: EntityId,
    pub dimensions: DimensionConfigMap,
    pub entities: EntityConfigMap,
    pub jobs: JobConfigMap,
}
