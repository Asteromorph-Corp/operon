use crate::meta_storage::{MetaBackendOptions, MetaStorageOptions};
use crate::ui::UiMode;

pub struct SchedulerOptions {
    pub(crate) internal_channel_size: usize,
    pub(crate) ui_mode: UiMode,
    pub(crate) backend: MetaBackendOptions,
    pub(crate) pool_size: usize,
    pub(crate) schema: Option<String>,
    pub(crate) keepalives_idle: std::time::Duration,
    pub(crate) keepalives_interval: std::time::Duration,
}

impl SchedulerOptions {
    pub(crate) fn split(self) -> (usize, UiMode, MetaStorageOptions) {
        let meta_storage_options = MetaStorageOptions {
            backend: self.backend,
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
