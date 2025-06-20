use crate::{
    scheduler::ControlEvent,
    ui::{LogRecord, Progress},
};

pub enum UiStateUpdate {
    ProgressUpdate(String, Progress),
    /// New log record.
    NewLog(LogRecord, u16),
    SetCursor(usize),
    ExitOnFinish(bool),
    LastControlEvent(ControlEvent),
}

impl From<ControlEvent> for UiStateUpdate {
    fn from(event: ControlEvent) -> Self {
        UiStateUpdate::LastControlEvent(event)
    }
}
