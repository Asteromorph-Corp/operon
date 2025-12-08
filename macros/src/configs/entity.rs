use indexmap::IndexMap;

pub type EntityId = String;

/// An Operon entity, which can be a struct or an enum.
#[derive(Debug, Clone)]
pub struct EntityConfig {
    /// Unique identifier for the entity.
    pub id: EntityId,
    /// Dimensions this entity repeat on.
    pub dims: Vec<syn::Ident>,
}

pub type EntityConfigMap = IndexMap<EntityId, EntityConfig>;
