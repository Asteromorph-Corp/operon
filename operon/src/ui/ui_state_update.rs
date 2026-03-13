use crate::scheduler::ControlEvent;
use crate::ui::Progress;

pub enum UiStateUpdate {
    ProgressUpdate(String, Progress),
    LastControlEvent(ControlEvent),
}

impl From<ControlEvent> for UiStateUpdate {
    fn from(event: ControlEvent) -> Self {
        UiStateUpdate::LastControlEvent(event)
    }
}
