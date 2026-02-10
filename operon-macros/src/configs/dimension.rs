use indexmap::IndexMap;

/// An Operon dimension.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DimensionConfig {
    /// Unique identifier for the dimension.
    pub id: syn::Ident,
    /// Upstream dimensions that this dimension depends on.
    pub depends_on: Vec<syn::Ident>,
}

pub type DimensionConfigMap = IndexMap<syn::Ident, DimensionConfig>;
