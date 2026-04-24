use async_trait::async_trait;
use tokio::time::Instant;
use uuid::Uuid;

use crate::scheduler::SchedulerError;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::events::ControlEvent;
use crate::scheduler::states::clean::CleanTransition;
use crate::scheduler::states::rebuild::RebuildTransition;
use crate::scheduler::states::start::StartTransition;
use crate::scheduler::states::{NextState, SchedulerState, TransitionState};
use crate::schema::CheckMode;
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::UiMode;

pub struct StaleState<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    ctx: SchedulerContext<Svc, Sto>,
    channel_size: usize,
    run_id: Uuid,
    kind: StaleKind,
    /// The result of the consistent check.
    is_consistent: Option<bool>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StaleKind {
    Complete,
    GracefulStop,
    Abort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Clean,
    Rebuild,
    Restore,
}

impl<Svc, Sto> StaleState<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn new(
        ctx: SchedulerContext<Svc, Sto>,
        ui_mode: UiMode,
        channel_size: usize,
        run_id: Uuid,
        kind: StaleKind,
    ) -> Self {
        match (ui_mode, kind) {
            (UiMode::Headless, _) => tracing::info!("Restoration is not supported in headless mode."),
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
            is_consistent: None,
        }
    }

    fn into_clean(self) -> TransitionState {
        CleanTransition::state(self.ctx, self.channel_size, self.run_id)
    }

    fn into_rebuild(self) -> TransitionState {
        RebuildTransition::state(self.ctx, self.channel_size, self.run_id)
    }

    fn into_restore(self) -> TransitionState {
        StartTransition::state(self.ctx, self.channel_size, self.run_id, false)
    }

    async fn run_consistency_check(&mut self, mode: CheckMode) -> Result<(), SchedulerError> {
        let check_start = Instant::now();
        let is_consistent = self
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
        self.is_consistent = Some(is_consistent);

        if !is_consistent {
            tracing::info!(
                "Some data is corrupted or missing.\n\
                Type `run` to start a new run and overwrite the existing data, or `exit` to cancel."
            );
            return Ok(());
        }

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
        Ok(())
    }

    fn choose_run_mode(&self, fresh: bool, rebuild: bool) -> Option<RunMode> {
        if fresh {
            tracing::info!("Starting a fresh run, ignoring previous data.");
            return Some(RunMode::Clean);
        }

        if rebuild {
            return match (&self.kind, self.is_consistent) {
                // Consistency check failed.
                (_, Some(false)) => {
                    tracing::error!("Cannot rebuild because of missing data.");
                    None
                }
                // Previous run was aborted, and no consistency check was performed.
                (StaleKind::Abort, None) => {
                    tracing::error!("Cannot rebuild before checking for consistency.");
                    None
                }
                // Previous run was either aborted but consistent, gracefully stopped, or complete.
                (StaleKind::Abort, Some(true))
                | (StaleKind::GracefulStop, _)
                | (StaleKind::Complete, _) => {
                    tracing::info!("Rebuilding the run from trusted data.");
                    Some(RunMode::Rebuild)
                }
            };
        }

        // The behaviour of `run` command without any flags.
        match (&self.kind, self.is_consistent) {
            // Consistency check failed.
            (_, Some(false)) => Some(RunMode::Clean),
            // Previous run was aborted, and no consistency check was performed.
            (StaleKind::Abort, None) => Some(RunMode::Clean),
            // Previous run was complete.
            (StaleKind::Complete, _) => Some(RunMode::Clean),
            // Previous run was aborted, but consistency check succeeded.
            (StaleKind::Abort, Some(true)) => {
                tracing::info!("Rebuilding the run from trusted data.");
                Some(RunMode::Rebuild)
            }
            // Previous run was gracefully stopped.
            (StaleKind::GracefulStop, _) => {
                tracing::info!("Continuing the last run.");
                Some(RunMode::Restore)
            }
        }
    }
}

#[async_trait]
impl<Svc, Sto> SchedulerState for StaleState<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    async fn handle_progress(
        self: Box<Self>,
    ) -> Result<NextState, crate::scheduler::SchedulerError> {
        Ok(NextState::Next(self))
    }

    async fn handle_control_event(
        mut self: Box<Self>,
        evt: ControlEvent,
    ) -> Result<NextState, crate::scheduler::SchedulerError> {
        match evt {
            ControlEvent::Check { .. } if self.is_consistent.is_some() => {
                tracing::warn!("Already run a check.")
            }
            ControlEvent::Check { mode } => {
                tracing::info!("Starting a consistency check of the remaining data.");
                self.run_consistency_check(mode).await?
            }
            ControlEvent::Run { fresh, rebuild } => match self.choose_run_mode(fresh, rebuild) {
                Some(RunMode::Clean) => return Ok(NextState::from(self.into_clean())),
                Some(RunMode::Rebuild) => return Ok(NextState::from(self.into_rebuild())),
                Some(RunMode::Restore) => return Ok(NextState::from(self.into_restore())),
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
