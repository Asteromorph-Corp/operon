use std::borrow::Cow;

use crate::{meta_storage::MetaStorageOptions, ui::LogOptions};

pub struct OperonOptions {
    // Storage options
    pub storage_uri: String,
    pub storage_pool_size: u32,
    pub storage_schema: Option<String>,
    pub storage_connection_timeout: std::time::Duration,
    // Meta storage options
    pub meta_storage_uri: String,
    pub meta_storage_pool_size: u32,
    pub meta_storage_schema: Option<String>,
    pub meta_storage_connection_timeout: std::time::Duration,
    // Log options
    pub log_level: log::Level,
    pub log_buffer_size: usize,
    pub log_dump: Option<String>,
}

impl OperonOptions {
    pub fn split(self) -> (MetaStorageOptions, LogOptions) {
        let meta_storage_options = MetaStorageOptions {
            database_uri: self.meta_storage_uri,
            pool_size: self.meta_storage_pool_size as usize,
            connection_timeout: self.meta_storage_connection_timeout,
            schema: self.meta_storage_schema,
        };

        let log_options = LogOptions {
            level: self.log_level,
            buffer_size: self.log_buffer_size,
            dump: self.log_dump.map(Cow::from),
        };

        (meta_storage_options, log_options)
    }
}
