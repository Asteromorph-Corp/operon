impl From<operon::options::MemStorageOptions> for MemCookingStorage {
    fn from(_options: operon::options::MemStorageOptions) -> Self {
        Self::default()
    }
}
