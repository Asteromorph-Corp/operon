use std::sync::Arc;

use indexmap::IndexMap;
use tokio::sync::RwLock;

#[derive(Default, Debug, Clone)]
#[repr(transparent)]
pub struct SharedProgressMap(pub IndexMap<&'static str, SharedProgress>);

#[derive(Default, Debug, Clone)]
#[repr(transparent)]
pub struct ProgressMap(pub IndexMap<&'static str, Progress>);

pub type SharedProgress = Arc<RwLock<Progress>>;

#[derive(Default, Debug, Clone, Copy)]
pub struct Progress {
    pub done: i64,
    pub queued: i64,
    pub waiting: i64,
    pub state: TaskState,
}

/// State of either an `IndividualScheduler` or the whole Operon.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Finished,
    #[default]
    Running,
    Paused,
    Error,
    Stopped,
}

impl SharedProgressMap {
    pub fn from_tasks(task_ids: &[&'static str]) -> Self {
        let progress = task_ids
            .iter()
            .map(|task_id| (*task_id, Arc::new(RwLock::new(Progress::default()))))
            .collect();
        Self(progress)
    }

    pub async fn snapshot(&self) -> ProgressMap {
        let futures: Vec<_> = self
            .0
            .iter()
            .map(|(id, shared)| async { (*id, *shared.read().await) })
            .collect();
        let results = futures::future::join_all(futures).await;
        ProgressMap(results.into_iter().collect())
    }
}

impl ProgressMap {
    fn state_iter(&self) -> impl Iterator<Item = TaskState> {
        self.0.values().map(|p| p.state)
    }

    pub fn overall_state(&self) -> TaskState {
        if self.any_error() {
            TaskState::Error
        } else if self.all_finished() {
            TaskState::Finished
        } else if self.all_paused() {
            TaskState::Paused
        } else if self.all_stopped() {
            TaskState::Stopped
        } else {
            TaskState::Running
        }
    }

    pub fn any_error(&self) -> bool {
        self.state_iter().any(|s| s == TaskState::Error)
    }

    pub fn all_finished(&self) -> bool {
        self.state_iter().all(|s| s == TaskState::Finished)
    }

    pub fn all_paused(&self) -> bool {
        self.state_iter()
            .all(|s| s == TaskState::Paused || s == TaskState::Finished)
    }

    pub fn all_stopped(&self) -> bool {
        self.state_iter()
            .all(|s| s == TaskState::Stopped || s == TaskState::Finished)
    }

    pub fn any_running(&self) -> bool {
        self.state_iter().any(|s| s == TaskState::Running)
    }

    pub fn any_paused(&self) -> bool {
        self.state_iter().any(|s| s == TaskState::Paused)
    }
}

impl Progress {
    pub fn new(done: i64, queued: i64, waiting: i64, state: TaskState) -> Self {
        Self {
            done,
            queued,
            waiting,
            state,
        }
    }

    /// Updates the progress counters and automatically transitions to `Finished`
    /// when no queued or waiting tasks remain.
    ///
    /// Returns `true` if the state is `Finished` after this update.
    pub fn update(&mut self, done: i64, queued: i64, waiting: i64) -> bool {
        self.done = done;
        self.queued = queued;
        self.waiting = waiting;

        if queued + waiting == 0 && self.state != TaskState::Finished {
            self.state = TaskState::Finished;
        }
        self.state == TaskState::Finished
    }

    /// Sets the execution state, unless the progress has already finished.
    pub fn set_state(&mut self, state: TaskState) {
        if self.state == TaskState::Finished {
            return;
        }
        self.state = state;
    }
}

impl TaskState {
    pub fn color(&self) -> ::ratatui::style::Color {
        match self {
            TaskState::Finished => ::ratatui::style::Color::Green,
            TaskState::Running => ::ratatui::style::Color::Cyan,
            TaskState::Paused => ::ratatui::style::Color::Yellow,
            TaskState::Error => ::ratatui::style::Color::Red,
            TaskState::Stopped => ::ratatui::style::Color::DarkGray,
        }
    }
}
