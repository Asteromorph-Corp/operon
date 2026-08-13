use std::sync::Arc;

use crate::meta_storage::{MetaBackend, MetaClientApi, MetaConnApi, MetaTxApi};
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::events::{ControlEventReceiver, SchedulerStateSender};
use crate::scheduler::states::{InitTransition, NextState, SchedulerState};
use crate::scheduler::{SchedulerError, SchedulerHandler};
use crate::schema::{RunFootprint, RunState, SharedProgressMap, TableShape};
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::UiMode;

/// The scheduler's result over its composite service/storage/metadata error.
type SchedulerResult<T, UErr, SErr, MErr> = Result<T, SchedulerError<UErr, SErr, MErr>>;

/// A boxed scheduler state keyed by the scheduler's composite error.
type BoxedState<UErr, SErr, MErr> =
    Box<dyn SchedulerState<Error = SchedulerError<UErr, SErr, MErr>>>;

/// # Scheduler
///
/// The orchestrating scheduler that manages the individual schedulers.
pub struct Scheduler<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    ctx: SchedulerContext<Svc, Sto, MSto>,
    ctrl_rx: ControlEventReceiver,
    channel_size: usize,
    ui_mode: UiMode,
}

#[allow(clippy::too_many_arguments)]
impl<Svc, Sto, MSto> Scheduler<Svc, Sto, MSto>
where
    Sto: OperonStorage,
    Svc: OperonService,
    MSto: MetaBackend,
{
    /// Initialize a new scheduler with the given components.
    pub fn new(
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: MSto,
        handler: SchedulerHandler<Svc, Sto, MSto>,
        progresses: SharedProgressMap,
        ctrl_rx: ControlEventReceiver,
        channel_size: usize,
        ui_mode: UiMode,
    ) -> Self {
        handler.validate_pool_sizes();

        let ctx = SchedulerContext {
            service,
            storage,
            meta_storage,
            handler,
            progresses,
        };

        Self {
            ctx,
            ctrl_rx,
            channel_size,
            ui_mode,
        }
    }

    /// Entry point for the scheduler.
    /// Erases the error type and sends the scheduler's result into `sched_tx`.
    ///
    /// Always sends a result unless the scheduler panics.
    pub async fn work_and_send(self, sched_tx: SchedulerStateSender) {
        let result = self.work().await;
        let _ = sched_tx.send(result.map_err(|e| e.to_string()));
    }

    /// Main work loop for the scheduler.
    /// Runs until the scheduler is finished or an error occurs.
    /// Returns whether the UI should exit or not.
    async fn work(mut self) -> SchedulerResult<bool, Svc::Error, Sto::Error, MSto::Error> {
        self.ctx.meta_storage.ensure_lock().await?;
        self.ctx.storage.init().await?;
        let shape_changed = self.init_meta_storage().await?.is_stale;
        if shape_changed {
            self.abort_recorded_run().await?;
        }

        let heartbeat_handle = self.ctx.meta_storage.clone();
        let mut state: BoxedState<Svc::Error, Sto::Error, MSto::Error> = Box::new(
            InitTransition::state(self.ctx, self.ui_mode, self.channel_size, shape_changed),
        );

        // Heartbeat tick
        let heartbeat_period = std::time::Duration::from_secs(30);
        let mut lock_heartbeat = tokio::time::interval_at(
            tokio::time::Instant::now() + heartbeat_period,
            heartbeat_period,
        );

        loop {
            let next = tokio::select! {
                result = state.wait_progress() => {
                    result?;
                    state.handle_progress().await?
                }
                Some(evt) = self.ctrl_rx.recv() => state.handle_control_event(evt).await?,
                _ = lock_heartbeat.tick() => {
                    heartbeat_handle.check_lock().await?;
                    continue;
                }
            };

            match next {
                NextState::Next(new_state) => state = new_state,
                NextState::Exit { exit_ui } => {
                    return Ok(exit_ui);
                }
            }
        }
    }

    /// An helper function to call `self.spec.init_meta_storage` with a transaction.
    ///
    /// Returns [`STALE`](TableShape::STALE) if any task or dimension changed shape, discarding what
    /// its table held.
    async fn init_meta_storage(
        &self,
    ) -> SchedulerResult<TableShape, Svc::Error, Sto::Error, MSto::Error> {
        let mut conn = self.ctx.meta_storage.scheduler_conn().await?;
        let tx = conn.transaction().await?;
        let shape = self.ctx.handler.init_meta_storage(tx.as_client()).await?;
        tx.commit().await?;
        Ok(shape)
    }

    /// Records the last run as aborted, since a dropped-and-rebuilt table leaves its progress
    /// inconsistent.
    /// Writes to both storages' footprint tables.
    async fn abort_recorded_run(&self) -> SchedulerResult<(), Svc::Error, Sto::Error, MSto::Error> {
        let conn = self.ctx.meta_storage.scheduler_conn().await?;
        let Some(recorded) = conn.as_client().get_footprint().await? else {
            return Ok(());
        };

        let footprint = RunFootprint::new(recorded.metadata.run_id, RunState::Aborted);
        self.ctx.storage.put_footprint(&footprint).await?;
        conn.as_client().upsert_run(&footprint).await?;

        Ok(())
    }
}
