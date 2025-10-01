use indexmap::IndexMap;

pub type DimensionId = String;

/// An Operon dimension.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DimensionConfig {
    /// Unique identifier for the dimension.
    pub id: DimensionId,
    /// Upstream dimensions that this dimension depends on.
    pub depends_on: Vec<DimensionId>,
}

pub type DimensionConfigMap = IndexMap<DimensionId, DimensionConfig>;
