use async_trait::async_trait;

use crate::scheduler::context::SchedulerContext;
use crate::scheduler::states::{NextState, SchedulerState};
use crate::scheduler::{ControlEvent, SchedulerError};
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub struct RunningState<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    ctx: SchedulerContext<Svc, Sto>,
}

impl<Svc, Sto> RunningState<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn new(ctx: SchedulerContext<Svc, Sto>, channel_size: usize, clean: bool) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl<Svc, Sto> SchedulerState for RunningState<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    async fn handle_progress(self: Box<Self>) -> Result<NextState, SchedulerError> {
        todo!();
    }

    async fn handle_control_event(
        self: Box<Self>,
        _: ControlEvent,
    ) -> Result<NextState, SchedulerError> {
        todo!();
    }
}
