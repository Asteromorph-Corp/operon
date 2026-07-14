use std::collections::HashSet;

use async_trait::async_trait;
use tokio::time::Instant;
use uuid::Uuid;

use crate::meta_storage::{MetaBackend, MetaConnApi};
use crate::scheduler::SchedulerError;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::events::{ControlEvent, RunEventInner};
use crate::scheduler::states::clean::CleanTransition;
use crate::scheduler::states::rebuild::RebuildTransition;
use crate::scheduler::states::start::StartTransition;
use crate::scheduler::states::{NextState, SchedulerState, TransitionState};
use crate::schema::CheckMode;
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::UiMode;

pub struct StaleState<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    ctx: SchedulerContext<Svc, Sto, MSto>,
    channel_size: usize,
    run_id: Uuid,
    kind: StaleKind,
    /// The result of the consistency check.
    inconsistent_jobs: Option<Vec<&'static str>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StaleKind {
    Complete,
    GracefulStop,
    Abort,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunMode {
    Clean,
    Rebuild { skip: HashSet<String> },
    Restore,
}

impl<Svc, Sto, MSto> StaleState<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    pub fn new(
        ctx: SchedulerContext<Svc, Sto, MSto>,
        ui_mode: UiMode,
        channel_size: usize,
        run_id: Uuid,
        kind: StaleKind,
    ) -> Self {
        match (ui_mode, kind) {
            (UiMode::Headless, _) => {
                tracing::info!("Restoration is not supported in headless mode.")
            }
            (_, StaleKind::Complete) => tracing::info!(
                "Found a finished run.\n\
                Type `run` to begin running jobs and overwrite the existing data, or `exit` to cancel.\n\
                You can also type `check` to check the consistency of the data."
            ),
            (_, StaleKind::GracefulStop) => tracing::info!(
                "Found a gracefully stopped run.\n\
                Type `run` to resume running jobs from the last run, or `help` for additional options."
            ),
            (_, StaleKind::Abort) => tracing::info!(
                "Found an aborted run.\n\
                Type `check` to check if the data is recoverable, `run` to start a new run and \
                overwrite the existing data, or `help` for additional options."
            ),
        }

        Self {
            ctx,
            channel_size,
            run_id,
            kind,
            inconsistent_jobs: None,
        }
    }

    fn into_clean(self) -> TransitionState<MSto::Error> {
        CleanTransition::state(self.ctx, self.channel_size, self.run_id)
    }

    fn into_rebuild(self, skip: HashSet<String>) -> TransitionState<MSto::Error> {
        RebuildTransition::state(self.ctx, self.channel_size, self.run_id, skip)
    }

    fn into_restore(self) -> TransitionState<MSto::Error> {
        StartTransition::state(self.ctx, self.channel_size, self.run_id, false)
    }

    async fn run_consistency_check(
        &mut self,
        mode: CheckMode,
    ) -> Result<(), SchedulerError<MSto::Error>> {
        let check_start = Instant::now();
        let inconsistent_jobs = self
            .ctx
            .handler
            .check_consistency(
                &self.ctx.storage,
                self.ctx.meta_storage.scheduler_conn().await?.as_client(),
                mode,
            )
            .await
            .inspect_err(|e| tracing::error!("Failed to check data consistency: {e}"))?;

        tracing::info!(
            "Consistency check completed in: {:?}.",
            check_start.elapsed()
        );

        if !inconsistent_jobs.is_empty() {
            tracing::warn!(
                "Consistency check failed for the following jobs:\n\
                {inconsistent_jobs:?}\n\
                You may proceed to overwrite the existing data with `run`, or alternatively,\n\
                you may rebuild other jobs by specifying --redo-inconsistent-jobs.",
            );
        } else {
            match self.kind {
                StaleKind::Complete => tracing::info!(
                    "No inconsistencies were found.\n\
                    Type `run` to resume running jobs from the last run, or `help` for additional options."
                ),
                StaleKind::GracefulStop => tracing::info!(
                    "No inconsistencies were found.\n\
                    Type `run` to resume running jobs from the last run, or `help` for additional options."
                ),
                StaleKind::Abort => tracing::info!(
                    "The data is recoverable.\n\
                    Type `run` to rebuild and resume running jobs from the last run, or `help` for additional options."
                ),
            }
        }

        self.inconsistent_jobs = Some(inconsistent_jobs);
        Ok(())
    }

    fn choose_run_mode(&self, run_inner: RunEventInner) -> Option<RunMode> {
        match run_inner {
            RunEventInner::Fresh => {
                tracing::info!("Starting a fresh run, ignoring previous data.");
                Some(RunMode::Clean)
            }

            RunEventInner::Rebuild {
                mut skip,
                redo_inconsistent_jobs,
            } => match (&self.kind, &self.inconsistent_jobs, redo_inconsistent_jobs) {
                // Consistency check failed.
                (_, Some(inconsistent_jobs), false) if !inconsistent_jobs.is_empty() => {
                    tracing::error!(
                        "Cannot rebuild because of the previously found inconsistencies."
                    );
                    None
                }
                // Consistency check failed, but the user chose to redo inconsistent jobs.
                (_, Some(inconsistent_jobs), true) if !inconsistent_jobs.is_empty() => {
                    tracing::info!("Rebuilding the run while redoing inconsistent jobs.");
                    for job in inconsistent_jobs {
                        skip.insert(job.to_string());
                    }
                    Some(RunMode::Rebuild { skip })
                }
                // Previous run was aborted, and no consistency check was performed.
                (StaleKind::Abort, None, _) => {
                    tracing::error!("Cannot rebuild before checking for consistency.");
                    None
                }
                // Previous run was either aborted but consistent, gracefully stopped, or
                // complete.
                (StaleKind::Abort, _, _)
                | (StaleKind::GracefulStop, _, _)
                | (StaleKind::Complete, _, _) => {
                    tracing::info!("Rebuilding the run from trusted data.");
                    Some(RunMode::Rebuild { skip })
                }
            },

            RunEventInner::Unspecified {
                redo_inconsistent_jobs,
            } => match (&self.kind, &self.inconsistent_jobs, redo_inconsistent_jobs) {
                // Consistency check failed, and the user does not wish to preserve data.
                (_, Some(inconsistent_jobs), false) if !inconsistent_jobs.is_empty() => {
                    Some(RunMode::Clean)
                }
                // Consistency check failed, and the user wishes to rebuild as much as we can.
                (_, Some(inconsistent_jobs), true) if !inconsistent_jobs.is_empty() => {
                    tracing::info!("Rebuilding the run while redoing inconsistent jobs.");
                    Some(RunMode::Rebuild {
                        skip: HashSet::from_iter(inconsistent_jobs.iter().map(|s| s.to_string())),
                    })
                }
                // Previous run was aborted, and no consistency check was performed.
                (StaleKind::Abort, None, false) => Some(RunMode::Clean),
                // Previous run was aborted, no consistency check was performed, but the user wishes
                // to rebuild as much as we can.
                (StaleKind::Abort, None, true) => {
                    tracing::error!("Cannot rebuild before checking for consistency.");
                    None
                }
                // Previous run was aborted, and the consistency check succeeded.
                (StaleKind::Abort, Some(_), _) => {
                    tracing::info!("Rebuilding the run from trusted data.");
                    Some(RunMode::Rebuild {
                        skip: HashSet::new(),
                    })
                }
                // Previous run was complete.
                (StaleKind::Complete, _, _) => Some(RunMode::Clean),
                // Previous run was gracefully stopped.
                (StaleKind::GracefulStop, _, _) => {
                    tracing::info!("Continuing the last run.");
                    Some(RunMode::Restore)
                }
            },
        }
    }
}

#[async_trait]
impl<Svc, Sto, MSto> SchedulerState<MSto::Error> for StaleState<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    async fn handle_progress(
        self: Box<Self>,
    ) -> Result<NextState<MSto::Error>, crate::scheduler::SchedulerError<MSto::Error>> {
        Ok(NextState::Next(self))
    }

    async fn handle_control_event(
        mut self: Box<Self>,
        evt: ControlEvent,
    ) -> Result<NextState<MSto::Error>, crate::scheduler::SchedulerError<MSto::Error>> {
        match evt {
            ControlEvent::Check { .. } if self.inconsistent_jobs.is_some() => {
                tracing::warn!("Already run a check.")
            }
            ControlEvent::Check { mode } => {
                tracing::info!("Starting a consistency check of the remaining data.");
                self.run_consistency_check(mode).await?
            }
            ControlEvent::Run(run_inner) => match self.choose_run_mode(run_inner) {
                Some(RunMode::Clean) => return Ok(NextState::next(self.into_clean())),
                Some(RunMode::Rebuild { skip }) => {
                    return Ok(NextState::next(self.into_rebuild(skip)));
                }
                Some(RunMode::Restore) => return Ok(NextState::next(self.into_restore())),
                None => {}
            },
            ControlEvent::Pause { .. } => {
                tracing::warn!("Cannot pause before the run has started.")
            }
            ControlEvent::Resume { .. } => {
                tracing::warn!("Cannot resume before the run has started.")
            }
            ControlEvent::Quit { .. } => return Ok(NextState::Exit { exit_ui: true }),
            ControlEvent::Exit => return Ok(NextState::Exit { exit_ui: true }),
        }
        Ok(NextState::Next(self))
    }
}
