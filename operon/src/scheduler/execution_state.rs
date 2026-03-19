/// State of either an `IndividualScheduler` or the whole Operon.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionState {
    Finished,
    #[default]
    Running,
    Paused,
    Error,
    Stopped,
}

impl ExecutionState {
    pub fn color(&self) -> ::ratatui::style::Color {
        match self {
            ExecutionState::Finished => ::ratatui::style::Color::Green,
            ExecutionState::Running => ::ratatui::style::Color::Cyan,
            ExecutionState::Paused => ::ratatui::style::Color::Yellow,
            ExecutionState::Error => ::ratatui::style::Color::Red,
            ExecutionState::Stopped => ::ratatui::style::Color::DarkGray,
        }
    }
}
