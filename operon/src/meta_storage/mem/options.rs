/// Options for the in-memory metadata backend.
///
/// The store is volatile and has no connection or schema to configure, so it currently takes no
/// parameters.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct MemMetaStorageOptions {}

impl MemMetaStorageOptions {
    pub fn new() -> Self {
        Self {}
    }
}
