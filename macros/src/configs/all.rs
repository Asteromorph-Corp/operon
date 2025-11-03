use crate::configs::{DimensionConfigMap, EntityConfigMap, JobConfigMap};

#[derive(Debug)]
pub struct AllConfig {
    pub service_id: String,
    pub dimensions: DimensionConfigMap,
    pub entities: EntityConfigMap,
    pub jobs: JobConfigMap,
}
