use std::sync::Arc;

use crate::meta_storage::{MetaBackend, MetaConnApi, MetaTxApi};
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::events::{ControlEventReceiver, SchedulerStateSender};
use crate::scheduler::states::{InitTransition, NextState, SchedulerState};
use crate::scheduler::{SchedulerError, SchedulerHandler};
use crate::schema::SharedProgressMap;
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::UiMode;

/// # Scheduler
///
/// The orchestrating scheduler that manages the individual schedulers.
///
/// It is responsible for:
///
/// * Initialization of the metadata storage,
/// * initialization of the individual schedulers, and
/// * communication between the UI and the individual schedulers.
pub struct Scheduler<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    ctx: SchedulerContext<Svc, Sto, MSto>,
    ctrl_rx: ControlEventReceiver,
    sched_tx: SchedulerStateSender,
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
    /// Initialize a new scheduler and its associated storages.
    pub fn new(
        service: Arc<Svc>,
        storage: Arc<Sto>,
        handler: SchedulerHandler<Svc, Sto, MSto>,
        progresses: SharedProgressMap,
        ctrl_rx: ControlEventReceiver,
        sched_tx: SchedulerStateSender,
        channel_size: usize,
        ui_mode: UiMode,
        backend: MSto::Options,
    ) -> Result<Self, SchedulerError<MSto::Error>> {
        handler.validate_pool_sizes();
        let meta_storage = MSto::new(backend)?;

        let ctx = SchedulerContext {
            service,
            storage,
            meta_storage,
            handler,
            progresses,
        };

        Ok(Self {
            ctx,
            ctrl_rx,
            sched_tx,
            channel_size,
            ui_mode,
        })
    }

    /// Main entry point for the scheduler.
    pub async fn work(mut self) -> Result<(), SchedulerError<MSto::Error>> {
        self.ctx.meta_storage.ensure_lock().await?;
        self.ctx.storage.init().await?;
        self.init_meta_storage().await?;

        let heartbeat_handle = self.ctx.meta_storage.clone();
        let mut state: Box<dyn SchedulerState<MSto::Error>> = Box::new(InitTransition::state(
            self.ctx,
            self.ui_mode,
            self.channel_size,
        ));

        // Main work tick
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));
        // Heartbeat tick
        let heartbeat_period = std::time::Duration::from_secs(30);
        let mut lock_heartbeat = tokio::time::interval_at(
            tokio::time::Instant::now() + heartbeat_period,
            heartbeat_period,
        );

        loop {
            let next = tokio::select! {
                _ = interval.tick() => state.handle_progress().await?,
                Some(evt) = self.ctrl_rx.recv() => state.handle_control_event(evt).await?,
                _ = lock_heartbeat.tick() => {
                    heartbeat_handle
                        .check_lock()
                        .await?;
                    continue;
                }
            };

            match next {
                NextState::Next(new_state) => state = new_state,
                NextState::Exit { exit_ui } => {
                    if self.sched_tx.send(exit_ui).is_err() {
                        tracing::error!("UI exited before scheduler.")
                    };
                    break;
                }
            }
        }

        Ok(())
    }

    /// An helper function to call `self.spec.init_meta_storage` with a transaction.
    async fn init_meta_storage(&self) -> Result<(), SchedulerError<MSto::Error>> {
        let mut conn = self.ctx.meta_storage.scheduler_conn().await?;
        let tx = conn.transaction().await?;
        self.ctx.handler.init_meta_storage(tx.as_client()).await?;
        tx.commit().await?;
        Ok(())
    }
}
