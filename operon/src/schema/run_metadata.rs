use std::str::FromStr;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// The trace a run leaves behind, used to check that a restart picks up consistent data.
///
/// A finishing run writes the same footprint to the entity storage through
/// [`OperonStorage::put_footprint`](crate::OperonStorage::put_footprint) and to the metadata
/// storage.
/// The next run compares the two, and resumes only when they agree — timestamp included.
///
/// [`Uuid`], [`DateTime`], and [`Utc`] are re-exported from the `uuid` and `chrono` crates for
/// construction of the footprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFootprint {
    /// The run this footprint belongs to.
    pub metadata: RunMetadata,
    /// When the footprint was taken.
    pub at: DateTime<Utc>,
}

/// The identity of a run and how far it got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunMetadata {
    /// Distinguishes this run from other runs over the same data.
    pub run_id: Uuid,
    /// The state the run had reached when it was recorded.
    pub state: RunState,
}

/// How far a run got. Decides whether a restart resumes it or starts over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    /// No run has been recorded yet.
    Fresh,
    /// The run is under way.
    Running,
    /// The run stopped gracefully and is resumable immediately.
    Stopped,
    /// Every task of the run finished.
    Completed,
    /// The run stopped without recording a graceful stop, so its progress is not trusted.
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

impl Default for RunMetadata {
    fn default() -> Self {
        Self {
            run_id: Uuid::new_v4(),
            state: RunState::Fresh,
        }
    }
}

impl std::fmt::Display for RunState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state_str = match self {
            RunState::Fresh => "fresh",
            RunState::Running => "running",
            RunState::Stopped => "paused",
            RunState::Completed => "completed",
            RunState::Aborted => "aborted",
        };
        write!(f, "{}", state_str)
    }
}

impl FromStr for RunState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("fresh") {
            Ok(RunState::Fresh)
        } else if s.eq_ignore_ascii_case("running") {
            Ok(RunState::Running)
        } else if s.eq_ignore_ascii_case("paused") {
            Ok(RunState::Stopped)
        } else if s.eq_ignore_ascii_case("completed") {
            Ok(RunState::Completed)
        } else if s.eq_ignore_ascii_case("aborted") {
            Ok(RunState::Aborted)
        } else {
            Err(s.to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::fresh(RunState::Fresh)]
    #[case::running(RunState::Running)]
    #[case::paused(RunState::Stopped)]
    #[case::completed(RunState::Completed)]
    #[case::aborted(RunState::Aborted)]
    fn test_run_state_round_trip(#[case] state: RunState) {
        assert_eq!(RunState::from_str(&state.to_string()), Ok(state));
    }
}
