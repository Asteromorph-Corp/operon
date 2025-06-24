use crate::meta_storage::MetaStorageOptions;

pub struct SchedulerOptions {
    pub internal_channel_size: usize,
    pub database_uri: String,
    pub pool_size: usize,
    pub connection_timeout: std::time::Duration,
    pub schema: Option<String>,
}

impl SchedulerOptions {
    pub fn split(self) -> (usize, MetaStorageOptions) {
        let internal_channel_size = self.internal_channel_size;
        let meta_storage_options = MetaStorageOptions {
            database_uri: self.database_uri,
            pool_size: self.pool_size,
            connection_timeout: self.connection_timeout,
            schema: self.schema,
        };
        (internal_channel_size, meta_storage_options)
    }
}
