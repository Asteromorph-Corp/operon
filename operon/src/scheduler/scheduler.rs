use std::sync::Arc;

use crate::meta_storage::MetaStorage;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::states::{InitTransition, NextState, SchedulerState};
use crate::scheduler::{
    ControlEventReceiver, SchedulerError, SchedulerHandler, SchedulerOptions, SchedulerStateSender,
};
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
pub struct Scheduler<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    ctx: SchedulerContext<Svc, Sto>,
    ctrl_rx: ControlEventReceiver,
    sched_tx: SchedulerStateSender,
    channel_size: usize,
    ui_mode: UiMode,
}

#[allow(clippy::too_many_arguments)]
impl<Svc, Sto> Scheduler<Svc, Sto>
where
    Sto: OperonStorage,
    Svc: OperonService,
{
    /// Initialize a new scheduler and its associated storages.
    pub fn new(
        service: Arc<Svc>,
        storage: Arc<Sto>,
        handler: SchedulerHandler<Svc, Sto>,
        progresses: SharedProgressMap,
        ctrl_rx: ControlEventReceiver,
        sched_tx: SchedulerStateSender,
        options: SchedulerOptions,
    ) -> Result<Self, SchedulerError> {
        let (channel_size, ui_mode, meta_storage_options) = options.split();
        let meta_storage = MetaStorage::new(meta_storage_options)?;

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
    pub async fn work(mut self) -> Result<(), SchedulerError> {
        self.ctx.storage.init().await?;
        self.init_meta_storage().await?;

        let mut state: Box<dyn SchedulerState> = Box::new(InitTransition::state(
            self.ctx,
            self.ui_mode,
            self.channel_size,
        ));

        let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));
        loop {
            let next = tokio::select! {
                _ = interval.tick() => state.handle_progress().await?,
                Ok(()) = self.ctrl_rx.changed() => {
                    let evt = self.ctrl_rx.borrow_and_update().clone();
                    state.handle_control_event(evt).await?
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
    async fn init_meta_storage(&self) -> Result<(), SchedulerError> {
        let mut conn = self.ctx.meta_storage.conn().await?;
        let tx = conn.transaction().await?;
        self.ctx.handler.init_meta_storage(tx.as_client()).await?;
        tx.commit().await?;
        Ok(())
    }
}
