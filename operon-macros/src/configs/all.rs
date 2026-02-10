use crate::configs::{DimensionConfigMap, EntityConfigMap, JobConfigMap};

#[derive(Debug)]
pub struct AllConfig {
    pub service_id: syn::Ident,
    pub dimensions: DimensionConfigMap,
    pub entities: EntityConfigMap,
    pub jobs: JobConfigMap,
}
