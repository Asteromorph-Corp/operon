use async_trait::async_trait;
use uuid::Uuid;

use crate::scheduler::ControlEvent;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::states::running::RunningState;
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

    fn into_running(self) -> RunningState<Svc, Sto> {
        RunningState::new(self.ctx, self.channel_size, true)
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
        Ok(NextState::Next(self))
    }

    async fn handle_control_event(
        self: Box<Self>,
        evt: ControlEvent,
    ) -> Result<NextState, crate::scheduler::SchedulerError> {
        match evt {
            ControlEvent::Check { .. } => log::warn!("Cannot check on a fresh run"),
            ControlEvent::Run { rebuild: true, .. } => {
                log::error!("Cannot rebuild on a fresh run.")
            }
            ControlEvent::Run { .. } => return Ok(NextState::from(self.into_running())),
            ControlEvent::Pause { .. } => log::warn!("Cannot pause before the run has started."),
            ControlEvent::Resume { .. } => log::warn!("Cannot resume before the run has started."),
            ControlEvent::Quit { .. } => return Ok(NextState::Exit),
            ControlEvent::Exit => return Ok(NextState::Exit),
            // TODO: remove other events.
            _ => {}
        }
        Ok(NextState::Next(self))
    }
}
