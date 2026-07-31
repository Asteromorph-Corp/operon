use std::cmp::Ordering;
use std::hash::Hash;

/// Ordering direction for a priority dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Ascending,
    Descending,
}

impl Direction {
    /// Applies the direction to `ord`: no-op for `Descending` (higher values
    /// win in the max-heap), reversed for `Ascending` (lower values win).
    pub fn apply(self, ord: Ordering) -> Ordering {
        match self {
            Direction::Descending => ord,
            Direction::Ascending => ord.reverse(),
        }
    }
}

/// The metadata describing one task.
#[derive(Debug, Clone, Copy, Hash)]
pub struct JobMetadata<const N: usize> {
    pub id: &'static str,
    pub dims: [&'static str; N],
    pub spawn_dim: Option<&'static str>,
    pub priority: &'static [(&'static str, Direction)],
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
    /// Returns the metadata for the dimension that the task spawns, if any.
    pub fn spawn_dim_meta(&self) -> Option<DimensionMetadata<N>> {
        let id = self.spawn_dim?;
        let deps = self.dims;
        Some(DimensionMetadata { id, deps })
    }
}
