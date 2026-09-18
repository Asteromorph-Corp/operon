use crate::ui::UiMode;

pub(crate) struct SchedulerOptions {
    pub(crate) internal_channel_size: usize,
    pub(crate) ui_mode: UiMode,
}

impl SchedulerOptions {
    pub(crate) fn split(self) -> (usize, UiMode) {
        (self.internal_channel_size, self.ui_mode)
    }
}
