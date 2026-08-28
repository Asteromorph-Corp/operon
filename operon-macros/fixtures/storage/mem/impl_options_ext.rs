impl MemCookingStorageOptionsExt for operon::options::MemStorageOptions {
    fn build(self) -> MemCookingStorage {
        MemCookingStorage::new(self)
    }
}
