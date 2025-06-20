pub struct MetaStorageOptions {
    pub database_uri: String,
    pub pool_size: usize,
    pub connection_timeout: std::time::Duration,
    pub schema: Option<String>,
}
