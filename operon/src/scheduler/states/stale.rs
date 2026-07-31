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
    inconsistent_tasks: Option<Vec<&'static str>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StaleKind {
    Complete,
    GracefulStop,
    Abort { shape_changed: bool },
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
            (
                _,
                StaleKind::Abort {
                    shape_changed: false,
                },
            ) => tracing::info!(
                "Found an aborted run.\n\
                Type `check` to check if the data is recoverable, `run` to start a new run and \
                overwrite the existing data, or `help` for additional options."
            ),
            (
                _,
                StaleKind::Abort {
                    shape_changed: true,
                },
            ) => tracing::warn!(
                "The pipeline has changed since the previous run.\n\
                You may attempt to `check` and `run --rebuild` to restore progress from unchanged tasks.\n\
                Type `run` to start a new run and overwrite the existing data, or `help` for additional options."
            ),
        }

        Self {
            ctx,
            channel_size,
            run_id,
            kind,
            inconsistent_tasks: None,
        }
    }

    fn into_clean(self) -> TransitionState<SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        CleanTransition::state(self.ctx, self.channel_size, self.run_id)
    }

    fn into_rebuild(
        self,
        skip: HashSet<String>,
    ) -> TransitionState<SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        RebuildTransition::state(self.ctx, self.channel_size, self.run_id, skip)
    }

    fn into_restore(self) -> TransitionState<SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        StartTransition::state(self.ctx, self.channel_size, self.run_id, false)
    }

    async fn run_consistency_check(
        &mut self,
        mode: CheckMode,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        let check_start = Instant::now();
        let inconsistent_tasks = self
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

        if !inconsistent_tasks.is_empty() {
            tracing::warn!(
                "Consistency check failed for the following tasks:\n\
                {inconsistent_tasks:?}\n\
                You may proceed to overwrite the existing data with `run`, or alternatively,\n\
                you may rebuild other tasks by specifying --redo-inconsistent-jobs.",
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
                StaleKind::Abort { .. } => tracing::info!(
                    "The data is recoverable.\n\
                    Type `run` to rebuild and resume running jobs from the last run, or `help` for additional options."
                ),
            }
        }

        self.inconsistent_tasks = Some(inconsistent_tasks);
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
                redo_inconsistent_tasks,
            } => match (
                &self.kind,
                &self.inconsistent_tasks,
                redo_inconsistent_tasks,
            ) {
                // Consistency check failed.
                (_, Some(inconsistent_tasks), false) if !inconsistent_tasks.is_empty() => {
                    tracing::error!(
                        "Cannot rebuild because of the previously found inconsistencies."
                    );
                    None
                }
                // Consistency check failed, but the user chose to redo inconsistent tasks.
                (_, Some(inconsistent_tasks), true) if !inconsistent_tasks.is_empty() => {
                    tracing::info!("Rebuilding the run while redoing inconsistent tasks.");
                    for task in inconsistent_tasks {
                        skip.insert(task.to_string());
                    }
                    Some(RunMode::Rebuild { skip })
                }
                // Previous run was aborted, and no consistency check was performed.
                (StaleKind::Abort { .. }, None, _) => {
                    tracing::error!("Cannot rebuild before checking for consistency.");
                    None
                }
                // Previous run was either aborted but consistent, gracefully stopped, or
                // complete.
                (StaleKind::Abort { .. }, _, _)
                | (StaleKind::GracefulStop, _, _)
                | (StaleKind::Complete, _, _) => {
                    tracing::info!("Rebuilding the run from trusted data.");
                    Some(RunMode::Rebuild { skip })
                }
            },

            RunEventInner::Unspecified {
                redo_inconsistent_tasks,
            } => match (
                &self.kind,
                &self.inconsistent_tasks,
                redo_inconsistent_tasks,
            ) {
                // Consistency check failed, and the user does not wish to preserve data.
                (_, Some(inconsistent_tasks), false) if !inconsistent_tasks.is_empty() => {
                    Some(RunMode::Clean)
                }
                // Consistency check failed, and the user wishes to rebuild as much as we can.
                (_, Some(inconsistent_tasks), true) if !inconsistent_tasks.is_empty() => {
                    tracing::info!("Rebuilding the run while redoing inconsistent tasks.");
                    Some(RunMode::Rebuild {
                        skip: HashSet::from_iter(inconsistent_tasks.iter().map(|s| s.to_string())),
                    })
                }
                // Previous run was aborted, and no consistency check was performed.
                (StaleKind::Abort { .. }, None, false) => Some(RunMode::Clean),
                // Previous run was aborted, no consistency check was performed, but the user wishes
                // to rebuild as much as we can.
                (StaleKind::Abort { .. }, None, true) => {
                    tracing::error!("Cannot rebuild before checking for consistency.");
                    None
                }
                // Previous run was aborted, and the consistency check succeeded.
                (StaleKind::Abort { .. }, Some(_), _) => {
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
impl<Svc, Sto, MSto> SchedulerState for StaleState<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    type Error = SchedulerError<Svc::Error, Sto::Error, MSto::Error>;

    async fn handle_progress(self: Box<Self>) -> Result<NextState<Self::Error>, Self::Error> {
        Ok(NextState::Next(self))
    }

    async fn handle_control_event(
        mut self: Box<Self>,
        evt: ControlEvent,
    ) -> Result<NextState<Self::Error>, Self::Error> {
        match evt {
            ControlEvent::Check { .. } if self.inconsistent_tasks.is_some() => {
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
