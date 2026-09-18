/// Options for the in-memory storage.
///
/// The store is volatile and has no connection or schema to configure, so it takes no parameters.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct MemStorageOptions {}

impl MemStorageOptions {
    /// Constructs a new `MemStorageOptions` instance.
    pub fn new() -> Self {
        Self {}
    }
}
