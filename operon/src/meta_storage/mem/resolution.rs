use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::RwLock;

use crate::meta_storage::MetaResolutionApi;
use crate::meta_storage::mem::error::{MemMetaError, MemResult};
use crate::meta_storage::mem::store::MemStore;
use crate::schema::{DimensionMetadata, Resolution, TableShape};

/// One dimension's resolution table, mapping a coordinate to its upper bound.
#[derive(Debug, Default)]
pub(super) struct ResolutionTable {
    rows: RwLock<HashMap<Box<[usize]>, usize>>,
}

/// Helper struct for querying the in-memory resolutions of a dimension.
#[derive(Debug)]
pub struct MemResolutionQueryBuilder<'a, const N: usize> {
    store: &'a MemStore,
    dim_meta: DimensionMetadata<N>,
}

impl MemStore {
    /// Helper method to create a `MemResolutionQueryBuilder` for a resolution of given dimension.
    pub(super) fn resolution<const N: usize>(
        &self,
        dim_meta: DimensionMetadata<N>,
    ) -> MemResolutionQueryBuilder<'_, N> {
        MemResolutionQueryBuilder {
            store: self,
            dim_meta,
        }
    }
}

impl<const N: usize> MetaResolutionApi<N> for MemResolutionQueryBuilder<'_, N> {
    type Error = MemMetaError;

    async fn init(&self) -> MemResult<TableShape> {
        self.store.init_resolution_table(self.dim_meta.id)?;
        Ok(TableShape::CURRENT)
    }

    async fn clear(&self) -> MemResult<()> {
        if let Some(table) = self.store.resolution_table(self.dim_meta.id)? {
            table.rows.write()?.clear();
        }
        Ok(())
    }

    async fn get(&self, coordinate: [usize; N]) -> MemResult<Option<Resolution<N>>> {
        let Some(table) = self.store.resolution_table(self.dim_meta.id)? else {
            return Ok(None);
        };
        let rows = table.rows.read()?;
        Ok(rows
            .get(&coordinate[..])
            .map(|&ub| Resolution { coordinate, ub }))
    }

    async fn put(&self, resolution: Resolution<N>) -> MemResult<()> {
        let Some(table) = self.store.resolution_table(self.dim_meta.id)? else {
            return Ok(());
        };
        let mut rows = table.rows.write()?;
        if let Entry::Vacant(entry) = rows.entry(resolution.coordinate.into()) {
            let _ = entry.insert(resolution.ub);
        }
        Ok(())
    }

    async fn dump(&self) -> MemResult<Vec<Resolution<N>>> {
        let Some(table) = self.store.resolution_table(self.dim_meta.id)? else {
            return Ok(Vec::new());
        };
        let rows = table.rows.read()?;
        let resolutions = rows
            .iter()
            .map(|(coordinate, &ub)| Resolution {
                coordinate: std::array::from_fn(|i| coordinate[i]),
                ub,
            })
            .collect::<Vec<_>>();
        Ok(resolutions)
    }

    async fn hydrate(&self, resolutions: Vec<Resolution<N>>) -> MemResult<()> {
        let Some(table) = self.store.resolution_table(self.dim_meta.id)? else {
            return Ok(());
        };
        let mut rows = table.rows.write()?;
        rows.clear();
        for resolution in resolutions {
            let _ = rows.insert(resolution.coordinate.into(), resolution.ub);
        }
        Ok(())
    }
}
