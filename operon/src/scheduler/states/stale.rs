use async_trait::async_trait;
use uuid::Uuid;

use crate::scheduler::ControlEvent;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::states::{NextState, SchedulerState};
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub struct StaleState<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    ctx: SchedulerContext<Svc, Sto>,
    channel_size: usize,
    run_id: Uuid,
    kind: StaleKind,
    /// The result of the consistent check.
    is_consistent: Option<bool>,
}

pub enum StaleKind {
    Complete,
    GracefulStop,
    Abort,
}

impl<Svc, Sto> StaleState<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn new(
        ctx: SchedulerContext<Svc, Sto>,
        channel_size: usize,
        run_id: Uuid,
        kind: StaleKind,
    ) -> Self {
        match kind {
            StaleKind::Complete => log::info!(
                "Found a finished run.\n\
                Type `run` to begin running jobs and overwrite the existing data, or `exit` to cancel.\n\
                You can also type `check` to check the consistency of the data."
            ),
            StaleKind::GracefulStop => log::info!(
                "Found a gracefully stopped run.\n\
                Type `run` to resume running jobs from the last run, or `help` for additional options."
            ),
            StaleKind::Abort => log::info!(
                "Found an aborted run.\n\
                Type `check` to check if the data is recoverable, `run` to start a new run and \
                overwrite the existing data, or `help` for additional options."
            ),
        }

        Self {
            ctx,
            channel_size,
            run_id,
            kind,
            is_consistent: None,
        }
    }
}

#[async_trait]
impl<Svc, Sto> SchedulerState for StaleState<Svc, Sto>
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
        mut self: Box<Self>,
        _: ControlEvent,
    ) -> Result<NextState, crate::scheduler::SchedulerError> {
        todo!();
    }
}
