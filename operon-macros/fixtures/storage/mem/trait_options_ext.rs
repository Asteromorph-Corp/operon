/// Builds the in-memory entity storage for this pipeline from `MemStorageOptions`.
///
/// ```rust,ignore
/// let storage = MemStorageOptions::new().build();
/// ```
pub trait MemCookingStorageOptionsExt {
    /// Builds the in-memory entity storage described by these options.
    fn build(self) -> MemCookingStorage;
}
