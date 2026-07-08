use crate::meta_storage::MetaBackendOptions;
use crate::ui::UiMode;

pub struct SchedulerOptions {
    pub(crate) internal_channel_size: usize,
    pub(crate) ui_mode: UiMode,
    pub(crate) backend: MetaBackendOptions,
}

impl SchedulerOptions {
    pub(crate) fn split(self) -> (usize, UiMode, MetaBackendOptions) {
        (self.internal_channel_size, self.ui_mode, self.backend)
    }
}
