use crate::configs::{DimensionConfigMap, EntityConfigMap, TaskConfigMap};

#[derive(Debug)]
pub(crate) struct AllConfig {
    pub service_id: syn::Ident,
    pub dimensions: DimensionConfigMap,
    pub entities: EntityConfigMap,
    pub tasks: TaskConfigMap,
}
