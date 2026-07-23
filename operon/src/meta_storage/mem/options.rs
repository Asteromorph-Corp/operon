use crate::meta_storage::mem::MemMetaStorage;

/// Options for the in-memory metadata backend.
///
/// The store is volatile and has no connection or schema to configure, so it takes no parameters.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct MemMetaStorageOptions {}

impl MemMetaStorageOptions {
    pub fn new() -> Self {
        Self {}
    }

    /// Builds the in-memory metadata backend from these options.
    pub fn build(self) -> MemMetaStorage {
        MemMetaStorage::default()
    }
}
