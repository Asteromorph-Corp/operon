use crate::scheduler::ControlEvent;
use crate::ui::Progress;

pub enum UiStateUpdate {
    ProgressUpdate(String, Progress),
    SetProgressCursor(u16),
    ExitOnFinish(bool),
    LastControlEvent(ControlEvent),
}

impl From<ControlEvent> for UiStateUpdate {
    fn from(event: ControlEvent) -> Self {
        UiStateUpdate::LastControlEvent(event)
    }
}
