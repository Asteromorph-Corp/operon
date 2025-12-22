use std::str::FromStr;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// The "footprint" for a run used to check the consistency of run across restarts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFootprint {
    pub metadata: RunMetadata,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunMetadata {
    pub run_id: Uuid,
    pub state: RunState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Fresh,
    Running,
    Paused,
    Completed,
    Aborted,
}

impl RunFootprint {
    pub fn new(run_id: Uuid, state: RunState) -> Self {
        let metadata = RunMetadata::new(run_id, state);
        Self::from(metadata)
    }

    pub fn at(run_id: Uuid, state: RunState, at: DateTime<Utc>) -> Self {
        let metadata = RunMetadata::new(run_id, state);
        Self { metadata, at }
    }
}

impl From<RunMetadata> for RunFootprint {
    fn from(metadata: RunMetadata) -> Self {
        Self {
            metadata,
            at: Utc::now(),
        }
    }
}

impl RunMetadata {
    pub fn new(run_id: Uuid, state: RunState) -> Self {
        Self { run_id, state }
    }
}

impl std::fmt::Display for RunState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state_str = match self {
            RunState::Fresh => "fresh",
            RunState::Running => "running",
            RunState::Paused => "paused",
            RunState::Completed => "completed",
            RunState::Aborted => "aborted",
        };
        write!(f, "{}", state_str)
    }
}

impl FromStr for RunState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "running" => Ok(RunState::Running),
            "paused" => Ok(RunState::Paused),
            "completed" => Ok(RunState::Completed),
            "aborted" => Ok(RunState::Aborted),
            // TODO: handle this
            _ => Err(format!("Invalid run state: {}", s)),
        }
    }
}
