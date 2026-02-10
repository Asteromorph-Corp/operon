use indexmap::IndexMap;

/// An Operon entity, which can be a struct or an enum.
#[derive(Debug, Clone)]
pub struct EntityConfig {
    /// Unique identifier for the entity.
    pub id: syn::Ident,
    /// Dimensions this entity repeat on.
    pub dims: Vec<syn::Ident>,
}

pub type EntityConfigMap = IndexMap<syn::Ident, EntityConfig>;
