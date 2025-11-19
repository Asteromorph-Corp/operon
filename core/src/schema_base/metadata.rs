/// A metadata for a job.
#[derive(Debug, Clone, Copy)]
pub struct JobMetadata<const N: usize> {
    pub id: &'static str,
    pub dims: [&'static str; N],
    pub spawn_dim: Option<&'static str>,
}

impl<const N: usize> JobMetadata<N> {
    /// Returns the metadata for the dimension that the job spawns, if any.
    pub fn spawn_dim_meta(&self) -> Option<DimensionMetadata<N>> {
        self.spawn_dim.map(|dim| DimensionMetadata {
            id: dim,
            deps: self.dims,
        })
    }
}

/// A metadata for a dimension.
#[derive(Debug, Clone, Copy)]
pub struct DimensionMetadata<const N: usize> {
    pub id: &'static str,
    pub deps: [&'static str; N],
}
