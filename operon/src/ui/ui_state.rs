use indexmap::IndexMap;

use crate::scheduler::{ExecutionState, JobHandler};
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::{UiError, UiStateUpdate};

pub type ProgressMap = IndexMap<String, Progress>;
pub type Progress = (i64, i64, i64, ExecutionState);

// TODO: remove this
/// Minimal state that holds the information needed to render the UI.
#[derive(Default, Debug, Clone)]
pub struct UiState {
    // Done, queued, waiting, state, returned.
    pub(super) progress: ProgressMap,
}

impl UiState {
    pub fn from_jobs<Svc: OperonService, Sto: OperonStorage>(
        jobs: &[Box<dyn JobHandler<Svc, Sto>>],
    ) -> Self {
        let progress = jobs
            .iter()
            .map(|job| (job.job_id().to_string(), (0, 0, 0, ExecutionState::Running)))
            .collect();
        Self { progress }
    }

    pub fn progress_iter(&self) -> impl Iterator<Item = &Progress> {
        self.progress.values()
    }

    pub fn state_iter(&self) -> impl Iterator<Item = ExecutionState> {
        self.progress_iter().map(|s| s.3)
    }

    pub fn overall_state(&self) -> ExecutionState {
        if self.state_iter().any(|s| s == ExecutionState::Error) {
            ExecutionState::Error
        } else if self.state_iter().all(|s| s == ExecutionState::Finished) {
            ExecutionState::Finished
        } else if self
            .state_iter()
            .all(|s| s == ExecutionState::Paused || s == ExecutionState::Finished)
        {
            ExecutionState::Paused
        } else if self
            .state_iter()
            .all(|s| s == ExecutionState::Stopped || s == ExecutionState::Finished)
        {
            ExecutionState::Stopped
        } else {
            ExecutionState::Running
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
        }

        Ok(())
    }
}
