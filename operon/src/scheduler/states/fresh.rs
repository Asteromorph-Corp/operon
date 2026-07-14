use async_trait::async_trait;
use uuid::Uuid;

use crate::meta_storage::MetaBackend;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::events::{ControlEvent, RunEventInner};
use crate::scheduler::states::clean::CleanTransition;
use crate::scheduler::states::{NextState, SchedulerState, TransitionState};
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::UiMode;

pub struct FreshState<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    ctx: SchedulerContext<Svc, Sto, MSto>,
    channel_size: usize,
    run_id: Uuid,
}

impl<Svc, Sto, MSto> FreshState<Svc, Sto, MSto>
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

    fn into_running(self) -> TransitionState<MSto::Error> {
        CleanTransition::state(self.ctx, self.channel_size, self.run_id)
    }
}

#[async_trait]
impl<Svc, Sto, MSto> SchedulerState<MSto::Error> for FreshState<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    async fn handle_progress(
        self: Box<Self>,
    ) -> Result<NextState<MSto::Error>, crate::scheduler::SchedulerError<MSto::Error>> {
        Ok(NextState::Next(self))
    }

    async fn handle_control_event(
        self: Box<Self>,
        evt: ControlEvent,
    ) -> Result<NextState<MSto::Error>, crate::scheduler::SchedulerError<MSto::Error>> {
        match evt {
            ControlEvent::Check { .. } => tracing::warn!("Cannot check on a fresh run"),
            ControlEvent::Run(RunEventInner::Rebuild { .. }) => {
                tracing::error!("Cannot rebuild on a fresh run.")
            }
            ControlEvent::Run(..) => return Ok(NextState::next(self.into_running())),
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
