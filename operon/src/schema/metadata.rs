use std::hash::Hash;

/// A metadata for a job.
#[derive(Debug, Clone, Copy, Hash)]
pub struct JobMetadata<const N: usize> {
    pub id: &'static str,
    pub dims: [&'static str; N],
    pub spawn_dim: Option<&'static str>,
}

/// A metadata for an entity.
#[derive(Debug)]
pub struct EntityMetadata<const N: usize, T> {
    pub id: &'static str,
    pub dims: [&'static str; N],
    pub _phantom: std::marker::PhantomData<T>,
}

impl<const N: usize, T> Clone for EntityMetadata<N, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<const N: usize, T> Copy for EntityMetadata<N, T> {}
impl<const N: usize, T> Hash for EntityMetadata<N, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.dims.hash(state);
    }
}

/// A metadata for a dimension.
#[derive(Debug, Clone, Copy, Hash)]
pub struct DimensionMetadata<const N: usize> {
    pub id: &'static str,
    pub deps: [&'static str; N],
}

impl<const N: usize> JobMetadata<N> {
    /// Returns the metadata for the dimension that the job spawns, if any.
    pub fn spawn_dim_meta(&self) -> Option<DimensionMetadata<N>> {
        let id = self.spawn_dim?;
        let deps = self.dims;
        Some(DimensionMetadata { id, deps })
    }
}
