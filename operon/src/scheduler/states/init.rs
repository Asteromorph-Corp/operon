use async_trait::async_trait;
use uuid::Uuid;

use crate::scheduler::SchedulerError;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::states::fresh::FreshState;
use crate::scheduler::states::stale::{StaleKind, StaleState};
use crate::scheduler::states::{NextState, SchedulerTransition, TransitionState};
use crate::schema::{RunMetadata, RunState};
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::UiMode;

pub struct InitTransition<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    ctx: SchedulerContext<Svc, Sto>,
    channel_size: usize,
    ui_mode: UiMode,
}

impl<Svc, Sto> InitTransition<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn new(ctx: SchedulerContext<Svc, Sto>, ui_mode: UiMode, channel_size: usize) -> Self {
        Self {
            ctx,
            channel_size,
            ui_mode,
        }
    }

    pub fn state(
        ctx: SchedulerContext<Svc, Sto>,
        ui_mode: UiMode,
        channel_size: usize,
    ) -> TransitionState {
        TransitionState::new(Self::new(ctx, ui_mode, channel_size))
    }

    // TODO: rename states
    fn into_fresh(self, run_id: Uuid) -> FreshState<Svc, Sto> {
        FreshState::new(self.ctx, self.ui_mode, self.channel_size, run_id)
    }

    fn into_stale(self, run_id: Uuid, kind: StaleKind) -> StaleState<Svc, Sto> {
        StaleState::new(self.ctx, self.channel_size, run_id, kind)
    }

    /// Fetch the metadata of a previous run if it exists.
    async fn get_run_metadata(&self) -> Result<Option<RunMetadata>, SchedulerError> {
        let meta_conn = self.ctx.meta_storage.ui_conn().await?;

        // Get footprints from both storages.
        let data_footprint = self.ctx.storage.get_footprint().await?;
        let meta_footprint = meta_conn.as_client().get_footprint().await?;

        // Early return if the state can be inferred through the footprints.
        match (data_footprint, meta_footprint) {
            (Some(df), Some(mf)) if df == mf => return Ok(Some(mf.metadata)),
            (Some(df), Some(mf)) if df != mf => {
                tracing::warn!(
                    "Inconsistent footprints between data and metadata storage: data footprint: {df:?}, metadata footprint: {mf:?},\
                    treating the run as aborted."
                );
                return Ok(Some(RunMetadata::new(
                    mf.metadata.run_id,
                    RunState::Aborted,
                )));
            }
            (None, Some(_)) => tracing::info!(
                "Footprint found in metadata storage, but not in data storage, treating the run as fresh."
            ),
            _ => {}
        }
        Ok(None)
    }
}

#[async_trait]
impl<Svc, Sto> SchedulerTransition for InitTransition<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    fn warn_msg(&self) -> Option<&'static str> {
        None
    }

    async fn execute(self) -> Result<NextState, SchedulerError> {
        let RunMetadata { run_id, state } = self.get_run_metadata().await?.unwrap_or_default();

        if self.ui_mode == UiMode::Headless {
            return Ok(NextState::from(self.into_fresh(run_id)));
        }

        let next = match state {
            RunState::Fresh => NextState::from(self.into_fresh(run_id)),
            RunState::Completed => NextState::from(self.into_stale(run_id, StaleKind::Complete)),
            RunState::Paused => NextState::from(self.into_stale(run_id, StaleKind::GracefulStop)),
            RunState::Running | RunState::Aborted => {
                NextState::from(self.into_stale(run_id, StaleKind::Abort))
            }
        };

        Ok(next)
    }
}
