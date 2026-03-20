use crate::ui::Progress;

pub enum UiStateUpdate {
    ProgressUpdate(String, Progress),
}
