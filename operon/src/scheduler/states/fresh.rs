use async_trait::async_trait;
use uuid::Uuid;

use crate::scheduler::ControlEvent;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::states::start::StartTransition;
use crate::scheduler::states::{NextState, SchedulerState, TransitionState};
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
            UiMode::Headless => tracing::info!("Starting in headless mode."),
            UiMode::Interactive => tracing::info!("Type `run` to begin running jobs."),
        }

        Self {
            ctx,
            channel_size,
            run_id,
        }
    }

    fn into_running(self) -> TransitionState {
        StartTransition::state(self.ctx, self.channel_size, self.run_id, true)
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
            ControlEvent::Check { .. } => tracing::warn!("Cannot check on a fresh run"),
            ControlEvent::Run { rebuild: true, .. } => {
                tracing::error!("Cannot rebuild on a fresh run.")
            }
            ControlEvent::Run { .. } => return Ok(NextState::from(self.into_running())),
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
