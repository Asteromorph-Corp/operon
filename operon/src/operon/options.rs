use secrecy::SecretString;

use crate::logger::LoggerOptions;
use crate::scheduler::SchedulerOptions;
use crate::ui::{UiMode, UiOptions};

pub struct OperonOptions {
    // UI options
    pub(crate) ui_mode: UiMode,
    // Scheduler options
    pub(crate) internal_channel_size: usize,
    // Meta storage options
    pub(crate) meta_storage_uri: SecretString,
    pub(crate) meta_storage_pool_size: u32,
    pub(crate) meta_storage_schema: Option<String>,
    pub(crate) meta_storage_keepalives_idle: std::time::Duration,
    pub(crate) meta_storage_keepalives_interval: std::time::Duration,
    // Log options
    pub(crate) log_level: tracing::Level,
    pub(crate) log_buffer_size: usize,
    pub(crate) log_dump: Option<String>,
}

impl OperonOptions {
    pub fn new(meta_storage_uri: impl Into<String>) -> Self {
        Self {
            ui_mode: UiMode::Interactive,
            internal_channel_size: 1024,
            meta_storage_uri: SecretString::from(meta_storage_uri.into()),
            meta_storage_pool_size: 16,
            meta_storage_schema: Some("metadata".to_string()),
            meta_storage_keepalives_idle: std::time::Duration::from_secs(60),
            meta_storage_keepalives_interval: std::time::Duration::from_secs(30),
            log_level: tracing::Level::INFO,
            log_buffer_size: 1024,
            log_dump: None,
        }
    }

    pub fn with_ui_mode(mut self, mode: UiMode) -> Self {
        self.ui_mode = mode;
        self
    }

    pub fn with_internal_channel_size(mut self, size: usize) -> Self {
        self.internal_channel_size = size;
        self
    }

    pub fn with_meta_storage_pool_size(mut self, size: u32) -> Self {
        self.meta_storage_pool_size = size;
        self
    }

    pub fn with_meta_storage_schema(mut self, schema: impl Into<String>) -> Self {
        let schema = schema.into();

        if !schema.chars().all(|c| c.is_alphanumeric() || c == '_') {
            panic!("Schema name can only contain alphanumeric characters and underscores");
        } else if schema.is_empty() {
            panic!("Schema name cannot be empty");
        }

        self.meta_storage_schema = Some(schema);
        self
    }

    pub fn with_meta_storage_keepalives_idle(mut self, duration: std::time::Duration) -> Self {
        self.meta_storage_keepalives_idle = duration;
        self
    }

    pub fn with_meta_storage_keepalives_interval(mut self, duration: std::time::Duration) -> Self {
        self.meta_storage_keepalives_interval = duration;
        self
    }

    pub fn with_log_level(mut self, level: tracing::Level) -> Self {
        self.log_level = level;
        self
    }

    pub fn with_log_buffer_size(mut self, size: usize) -> Self {
        self.log_buffer_size = size;
        self
    }

    pub fn with_log_dump(mut self, dump: impl Into<String>) -> Self {
        self.log_dump = Some(dump.into());
        self
    }

    pub(crate) fn split(self) -> (UiOptions, SchedulerOptions, LoggerOptions) {
        let ui_options = UiOptions {
            mode: self.ui_mode,
            log_buffer_size: self.log_buffer_size,
        };
        let scheduler_options = SchedulerOptions {
            internal_channel_size: self.internal_channel_size,
            ui_mode: self.ui_mode,
            database_uri: self.meta_storage_uri,
            pool_size: self.meta_storage_pool_size as usize,
            schema: self.meta_storage_schema,
            keepalives_idle: self.meta_storage_keepalives_idle,
            keepalives_interval: self.meta_storage_keepalives_interval,
        };
        let log_options = LoggerOptions {
            level: self.log_level,
            buffer_size: self.log_buffer_size,
            dump: self.log_dump,
        };

        (ui_options, scheduler_options, log_options)
    }
}
