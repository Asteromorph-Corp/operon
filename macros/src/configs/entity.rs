use crate::configs::DimensionId;

pub type EntityId = String;

/// An Operon entity, which can be a struct or an enum.
#[derive(Debug)]
pub struct EntityConfig {
    /// Unique identifier for the entity.
    pub id: EntityId,
    /// Dimensions this entity repeat on.
    pub dims: Vec<DimensionId>,
    /// The Rust definition of this entity.
    pub body: String,
}
