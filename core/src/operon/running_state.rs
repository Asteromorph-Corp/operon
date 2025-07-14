/// State of either an `IndividualScheduler` or the whole Operon.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunningState {
    Finished,
    #[default]
    Running,
    Paused,
    Error,
    Stopped,
}

impl RunningState {
    pub fn color(&self) -> ::ratatui::style::Color {
        match self {
            RunningState::Finished => ::ratatui::style::Color::Green,
            RunningState::Running => ::ratatui::style::Color::Cyan,
            RunningState::Paused => ::ratatui::style::Color::Yellow,
            RunningState::Error => ::ratatui::style::Color::Red,
            RunningState::Stopped => ::ratatui::style::Color::DarkGray,
        }
    }
}
