use std::sync::Arc;

use indexmap::IndexMap;
use tokio::sync::RwLock;

use crate::scheduler::{ExecutionState, JobHandler};
use crate::service::OperonService;
use crate::storage::OperonStorage;

#[derive(Default, Debug, Clone)]
#[repr(transparent)]
pub struct SharedProgressMap(pub IndexMap<String, SharedProgress>);

#[derive(Default, Debug, Clone)]
#[repr(transparent)]
pub struct ProgressMap(pub IndexMap<String, Progress>);

pub type SharedProgress = Arc<RwLock<Progress>>;

#[derive(Default, Debug, Clone, Copy)]
pub struct Progress {
    pub done: i64,
    pub queued: i64,
    pub waiting: i64,
    pub state: ExecutionState,
}

impl SharedProgressMap {
    pub fn from_jobs<Svc: OperonService, Sto: OperonStorage>(
        jobs: &[Box<dyn JobHandler<Svc, Sto>>],
    ) -> Self {
        let progress = jobs
            .iter()
            .map(|job| {
                let id = job.job_id().to_owned();
                (id, Arc::new(RwLock::new(Progress::default())))
            })
            .collect();
        Self(progress)
    }

    pub async fn snapshot(&self) -> ProgressMap {
        let futures: Vec<_> = self
            .0
            .iter()
            .map(|(id, shared)| async { (id.clone(), *shared.read().await) })
            .collect();
        let results = futures::future::join_all(futures).await;
        ProgressMap(results.into_iter().collect())
    }
}

impl ProgressMap {
    fn state_iter(&self) -> impl Iterator<Item = ExecutionState> {
        self.0.values().map(|p| p.state)
    }

    pub fn overall_state(&self) -> ExecutionState {
        if self.any_error() {
            ExecutionState::Error
        } else if self.all_finished() {
            ExecutionState::Finished
        } else if self.all_paused() {
            ExecutionState::Paused
        } else if self.all_stopped() {
            ExecutionState::Stopped
        } else {
            ExecutionState::Running
        }
    }

    pub fn any_error(&self) -> bool {
        self.state_iter().any(|s| s == ExecutionState::Error)
    }

    pub fn all_finished(&self) -> bool {
        self.state_iter().all(|s| s == ExecutionState::Finished)
    }

    pub fn all_paused(&self) -> bool {
        self.state_iter()
            .all(|s| s == ExecutionState::Paused || s == ExecutionState::Finished)
    }

    pub fn all_stopped(&self) -> bool {
        self.state_iter()
            .all(|s| s == ExecutionState::Stopped || s == ExecutionState::Finished)
    }

    pub fn any_running(&self) -> bool {
        self.state_iter().any(|s| s == ExecutionState::Running)
    }

    pub fn any_paused(&self) -> bool {
        self.state_iter().any(|s| s == ExecutionState::Paused)
    }
}

impl Progress {
    pub fn new(done: i64, queued: i64, waiting: i64, state: ExecutionState) -> Self {
        Self {
            done,
            queued,
            waiting,
            state,
        }
    }
}
