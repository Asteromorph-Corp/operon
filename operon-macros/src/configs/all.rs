use crate::configs::{DimensionConfigMap, EntityConfigMap, TaskConfigMap};

#[derive(Debug)]
pub struct AllConfig {
    pub service_id: syn::Ident,
    pub dimensions: DimensionConfigMap,
    pub entities: EntityConfigMap,
    pub tasks: TaskConfigMap,
}
