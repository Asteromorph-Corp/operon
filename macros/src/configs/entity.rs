use indexmap::IndexMap;

use crate::configs::DimensionId;

pub type EntityId = String;

/// An Operon entity, which can be a struct or an enum.
#[derive(Debug, Clone)]
pub struct EntityConfig {
    /// Unique identifier for the entity.
    pub id: EntityId,
    /// Dimensions this entity repeat on.
    pub dims: Vec<DimensionId>,
}

pub type EntityConfigMap = IndexMap<EntityId, EntityConfig>;
