use indexmap::IndexMap;

use crate::operon::RunningState;
use crate::scheduler::{ControlEvent, JobHandler};
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::{UiError, UiStateUpdate};

pub type Progress = (i64, i64, i64, RunningState, bool);

/// Minimal state that holds the information needed to render the UI.
#[derive(Default, Debug, Clone)]
pub struct UiState {
    // Done, queued, waiting, state, returned.
    pub(super) progress: IndexMap<String, Progress>,
    /// Last sent control event.
    pub(super) last_control_event: ControlEvent,
}

impl UiState {
    pub fn from_jobs<Svc: OperonService, Sto: OperonStorage>(
        jobs: &[Box<dyn JobHandler<Svc, Sto>>],
    ) -> Self {
        let progress = jobs
            .iter()
            .map(|job| {
                (
                    job.job_id().to_string(),
                    (0, 0, 0, RunningState::Running, false),
                )
            })
            .collect();
        Self {
            progress,
            ..Default::default()
        }
    }

    pub fn progress_iter(&self) -> impl Iterator<Item = &Progress> {
        self.progress.values()
    }

    pub fn state_iter(&self) -> impl Iterator<Item = RunningState> {
        self.progress_iter().map(|s| s.3)
    }

    pub fn any_alive(&self) -> bool {
        self.progress_iter().any(|s| !s.4)
    }

    pub fn overall_state(&self) -> RunningState {
        if self.state_iter().any(|s| s == RunningState::Error) {
            RunningState::Error
        } else if self.state_iter().all(|s| s == RunningState::Finished) {
            RunningState::Finished
        } else if self
            .state_iter()
            .all(|s| s == RunningState::Paused || s == RunningState::Finished)
        {
            RunningState::Paused
        } else if self
            .state_iter()
            .all(|s| s == RunningState::Stopped || s == RunningState::Finished)
        {
            RunningState::Stopped
        } else {
            RunningState::Running
        }
    }

    pub fn update_ui_state(&mut self, update: impl Into<UiStateUpdate>) -> Result<(), UiError> {
        let update: UiStateUpdate = update.into();
        match update {
            UiStateUpdate::ProgressUpdate(id, progress) => {
                let Some(v) = self.progress.get_mut(&id) else {
                    return Err(UiError::ProgressNotFound(id));
                };
                *v = progress;
            }
            UiStateUpdate::LastControlEvent(event) => {
                self.last_control_event = event;
            }
        }

        Ok(())
    }
}
