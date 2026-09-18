use crate::logger::LoggerOptions;
use crate::scheduler::SchedulerOptions;
use crate::ui::{UiMode, UiOptions};

/// Surface-level settings for a run: UI mode, internal channel sizing, and logging.
pub struct OperonOptions {
    // UI options
    pub(super) ui_mode: UiMode,
    // Scheduler options
    pub(super) internal_channel_size: usize,
    // Log options
    pub(super) log_level: tracing::Level,
    pub(super) log_buffer_size: usize,
    pub(super) log_dump: Option<String>,
}

impl Default for OperonOptions {
    fn default() -> Self {
        Self {
            ui_mode: UiMode::Interactive,
            internal_channel_size: 1024,
            log_level: tracing::Level::INFO,
            log_buffer_size: 1024,
            log_dump: None,
        }
    }
}

impl OperonOptions {
    /// Creates run options with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ui_mode(mut self, mode: UiMode) -> Self {
        self.ui_mode = mode;
        self
    }

    pub fn with_internal_channel_size(mut self, size: usize) -> Self {
        self.internal_channel_size = size;
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

    pub(super) fn split(self) -> (UiOptions, SchedulerOptions, LoggerOptions) {
        let ui_options = UiOptions {
            mode: self.ui_mode,
            log_buffer_size: self.log_buffer_size,
        };
        let scheduler_options = SchedulerOptions {
            internal_channel_size: self.internal_channel_size,
            ui_mode: self.ui_mode,
        };
        let log_options = LoggerOptions {
            level: self.log_level,
            buffer_size: self.log_buffer_size,
            dump: self.log_dump,
        };

        (ui_options, scheduler_options, log_options)
    }
}
