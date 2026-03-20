use async_trait::async_trait;
use uuid::Uuid;

use crate::scheduler::ControlEvent;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::states::{NextState, SchedulerState};
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::UiMode;

pub struct FreshState<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    ctx: SchedulerContext<Svc, Sto>,
    channel_size: usize,
    run_id: Uuid,
}

impl<Svc, Sto> FreshState<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn new(
        ctx: SchedulerContext<Svc, Sto>,
        ui_mode: UiMode,
        channel_size: usize,
        run_id: Uuid,
    ) -> Self {
        match ui_mode {
            UiMode::Headless => log::info!("Starting in headless mode."),
            UiMode::Interactive => log::info!("Type `run` to begin running jobs."),
        }

        Self {
            ctx,
            channel_size,
            run_id,
        }
    }
}

#[async_trait]
impl<Svc, Sto> SchedulerState for FreshState<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    async fn handle_progress(
        self: Box<Self>,
    ) -> Result<NextState, crate::scheduler::SchedulerError> {
        todo!()
    }

    async fn handle_control_event(
        self: Box<Self>,
        _: ControlEvent,
    ) -> Result<NextState, crate::scheduler::SchedulerError> {
        todo!();
    }
}
