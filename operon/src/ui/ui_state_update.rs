use crate::scheduler::ControlEvent;
use crate::ui::{LogRecord, Progress};

pub enum UiStateUpdate {
    ProgressUpdate(String, Progress),
    /// New log record.
    NewLog(LogRecord, u16),
    SetLogCursor(usize),
    SetProgressCursor(u16),
    ExitOnFinish(bool),
    LastControlEvent(ControlEvent),
}

impl From<ControlEvent> for UiStateUpdate {
    fn from(event: ControlEvent) -> Self {
        UiStateUpdate::LastControlEvent(event)
    }
}
