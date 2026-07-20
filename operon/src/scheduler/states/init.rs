use async_trait::async_trait;
use uuid::Uuid;

use crate::meta_storage::{MetaBackend, MetaClientApi, MetaConnApi};
use crate::scheduler::SchedulerError;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::states::fresh::FreshState;
use crate::scheduler::states::stale::{StaleKind, StaleState};
use crate::scheduler::states::{NextState, SchedulerTransition, TransitionState};
use crate::schema::{RunMetadata, RunState};
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::UiMode;

pub struct InitTransition<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    ctx: SchedulerContext<Svc, Sto, MSto>,
    channel_size: usize,
    ui_mode: UiMode,
}

impl<Svc, Sto, MSto> InitTransition<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    pub fn new(
        ctx: SchedulerContext<Svc, Sto, MSto>,
        ui_mode: UiMode,
        channel_size: usize,
    ) -> Self {
        Self {
            ctx,
            channel_size,
            ui_mode,
        }
    }

    pub fn state(
        ctx: SchedulerContext<Svc, Sto, MSto>,
        ui_mode: UiMode,
        channel_size: usize,
    ) -> TransitionState<SchedulerError<MSto::Error, Sto::Error, Svc::Error>> {
        TransitionState::new(Self::new(ctx, ui_mode, channel_size))
    }

    // TODO: rename states
    fn into_fresh(self, run_id: Uuid) -> FreshState<Svc, Sto, MSto> {
        FreshState::new(self.ctx, self.ui_mode, self.channel_size, run_id)
    }

    fn into_stale(self, run_id: Uuid, kind: StaleKind) -> StaleState<Svc, Sto, MSto> {
        StaleState::new(self.ctx, self.ui_mode, self.channel_size, run_id, kind)
    }

    /// Fetch the metadata of a previous run if it exists.
    async fn get_run_metadata(
        &self,
    ) -> Result<Option<RunMetadata>, SchedulerError<MSto::Error, Sto::Error, Svc::Error>> {
        let meta_conn = self.ctx.meta_storage.scheduler_conn().await?;

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
impl<Svc, Sto, MSto> SchedulerTransition for InitTransition<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    type Error = SchedulerError<MSto::Error, Sto::Error, Svc::Error>;

    fn warn_msg(&self) -> Option<&'static str> {
        None
    }

    async fn execute(self) -> Result<NextState<Self::Error>, Self::Error> {
        let RunMetadata { run_id, state } = self.get_run_metadata().await?.unwrap_or_default();

        // if self.ui_mode == UiMode::Headless {
        //     return Ok(NextState::next(self.into_fresh(run_id)));
        // }

        let next = match state {
            RunState::Fresh => NextState::next(self.into_fresh(run_id)),
            RunState::Completed => NextState::next(self.into_stale(run_id, StaleKind::Complete)),
            RunState::Paused => NextState::next(self.into_stale(run_id, StaleKind::GracefulStop)),
            RunState::Running | RunState::Aborted => {
                NextState::next(self.into_stale(run_id, StaleKind::Abort))
            }
        };

        Ok(next)
    }
}
