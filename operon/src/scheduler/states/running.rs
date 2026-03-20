use async_trait::async_trait;
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::scheduler::context::SchedulerContext;
use crate::scheduler::states::{NextState, SchedulerState};
use crate::scheduler::{ControlEvent, ExecutionState, SchedulerError};
use crate::schema::{RunFootprint, RunState};
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub struct RunningState<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    ctx: SchedulerContext<Svc, Sto>,
    run_id: Uuid,
    execution_id: Uuid,
    handles: JoinSet<ExecutionState>,
    returned: Vec<ExecutionState>,
}

impl<Svc, Sto> RunningState<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn new(
        ctx: SchedulerContext<Svc, Sto>,
        channel_size: usize,
        run_id: Uuid,
        execution_id: Uuid,
        clean: bool,
    ) -> Self {
        let handles = ctx.handler.prepare_channels(channel_size).run_schedulers(
            &ctx.service,
            &ctx.storage,
            &ctx.meta_storage,
            &ctx.ui_state,
            &ctx.ctrl_rx,
            clean,
        );

        Self {
            ctx,
            handles,
            run_id,
            execution_id,
            returned: Vec::new(),
        }
    }
}

#[async_trait]
impl<Svc, Sto> SchedulerState for RunningState<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    async fn handle_progress(mut self: Box<Self>) -> Result<NextState, SchedulerError> {
        if let Some(res) = self.handles.try_join_next() {
            self.returned.push(res?);
        }

        if !self.handles.is_empty() {
            return Ok(NextState::Next(self));
        }

        let state = if self.returned.iter().all(|&s| s == ExecutionState::Finished) {
            RunState::Completed
        } else if self
            .returned
            .iter()
            .all(|&s| s == ExecutionState::Stopped || s == ExecutionState::Finished)
            && self.ctx.ctrl_rx.borrow().clone() == ControlEvent::GracefulStop
        {
            RunState::Paused
        } else {
            RunState::Aborted
        };

        let footprint = RunFootprint::new(self.run_id, state);

        // Write the footprint to the data storage...
        self.ctx.storage.put_footprint(&footprint).await?;

        // ...and to the metadata storage.
        let mut conn = self.ctx.meta_storage.conn().await?;
        let tx = conn.transaction().await?;
        tx.as_client().upsert_run(&footprint).await?;
        tx.as_client()
            .update_execution_on_finish(&footprint, self.execution_id)
            .await?;
        tx.commit().await?;

        Ok(NextState::Exit)
    }

    async fn handle_control_event(
        self: Box<Self>,
        _: ControlEvent,
    ) -> Result<NextState, SchedulerError> {
        Ok(NextState::Next(self))
    }
}
