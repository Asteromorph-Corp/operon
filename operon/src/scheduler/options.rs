use secrecy::SecretString;

use crate::meta_storage::MetaStorageOptions;
use crate::ui::UiMode;

pub struct SchedulerOptions {
    pub(crate) internal_channel_size: usize,
    pub(crate) ui_mode: UiMode,
    pub(crate) database_uri: SecretString,
    pub(crate) pool_size: usize,
    pub(crate) schema: Option<String>,
    pub(crate) keepalives_idle: std::time::Duration,
    pub(crate) keepalives_interval: std::time::Duration,
}

impl SchedulerOptions {
    pub(crate) fn split(self) -> (usize, UiMode, MetaStorageOptions) {
        let meta_storage_options = MetaStorageOptions {
            database_uri: self.database_uri,
            pool_size: self.pool_size,
            keepalives_idle: self.keepalives_idle,
            keepalives_interval: self.keepalives_interval,
            schema: self.schema,
        };
        (
            self.internal_channel_size,
            self.ui_mode,
            meta_storage_options,
        )
    }
}
