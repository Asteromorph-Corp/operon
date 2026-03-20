use async_trait::async_trait;
use uuid::Uuid;

use crate::scheduler::SchedulerError;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::states::running::RunningState;
use crate::scheduler::states::{NextState, SchedulerTransition, TransitionState};
use crate::schema::{RunFootprint, RunState};
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub struct StartTransition<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    ctx: SchedulerContext<Svc, Sto>,
    channel_size: usize,
    run_id: Uuid,
    clean: bool,
}

impl<Svc, Sto> StartTransition<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn new(
        ctx: SchedulerContext<Svc, Sto>,
        channel_size: usize,
        run_id: Uuid,
        clean: bool,
    ) -> Self {
        Self {
            ctx,
            channel_size,
            run_id,
            clean,
        }
    }

    pub fn state(
        ctx: SchedulerContext<Svc, Sto>,
        channel_size: usize,
        run_id: Uuid,
        clean: bool,
    ) -> TransitionState {
        TransitionState::new(Self::new(ctx, channel_size, run_id, clean))
    }

    fn into_running(self, execution_id: Uuid) -> RunningState<Svc, Sto> {
        RunningState::new(
            self.ctx,
            self.channel_size,
            self.run_id,
            execution_id,
            false,
        )
    }
}

#[async_trait]
impl<Svc, Sto> SchedulerTransition for StartTransition<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    fn warn_msg(&self) -> Option<&'static str> {
        None
    }

    async fn execute(self) -> Result<NextState, SchedulerError> {
        let footprint = RunFootprint::new(self.run_id, RunState::Running);
        let execution_id = Uuid::new_v4();

        let conn = self.ctx.meta_storage.conn().await?;

        conn.as_client().upsert_run(&footprint).await?;
        conn.as_client()
            .put_execution(footprint.metadata.run_id, execution_id)
            .await?;

        Ok(NextState::from(self.into_running(execution_id)))
    }
}
