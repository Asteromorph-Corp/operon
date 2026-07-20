use async_trait::async_trait;
use uuid::Uuid;

use crate::meta_storage::{MetaBackend, MetaClientApi, MetaConnApi};
use crate::scheduler::SchedulerError;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::states::running::RunningState;
use crate::scheduler::states::{NextState, SchedulerTransition, TransitionState};
use crate::schema::{RunFootprint, RunState};
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub struct StartTransition<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    ctx: SchedulerContext<Svc, Sto, MSto>,
    channel_size: usize,
    run_id: Uuid,
    clean: bool,
}

impl<Svc, Sto, MSto> StartTransition<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    pub fn new(
        ctx: SchedulerContext<Svc, Sto, MSto>,
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
        ctx: SchedulerContext<Svc, Sto, MSto>,
        channel_size: usize,
        run_id: Uuid,
        clean: bool,
    ) -> TransitionState<SchedulerError<MSto::Error, Sto::Error, Svc::Error>> {
        TransitionState::new(Self::new(ctx, channel_size, run_id, clean))
    }

    fn into_running(self, execution_id: Uuid) -> RunningState<Svc, Sto, MSto> {
        RunningState::new(
            self.ctx,
            self.channel_size,
            self.run_id,
            execution_id,
            self.clean,
        )
    }
}

#[async_trait]
impl<Svc, Sto, MSto> SchedulerTransition for StartTransition<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    type Error = SchedulerError<MSto::Error, Sto::Error, Svc::Error>;

    fn warn_msg(&self) -> Option<&'static str> {
        None
    }

    async fn execute(self) -> Result<NextState<Self::Error>, Self::Error> {
        let footprint = RunFootprint::new(self.run_id, RunState::Running);
        let execution_id = Uuid::new_v4();

        let conn = self.ctx.meta_storage.scheduler_conn().await?;

        conn.as_client().upsert_run(&footprint).await?;
        conn.as_client()
            .put_execution(footprint.metadata.run_id, execution_id)
            .await?;

        Ok(NextState::next(self.into_running(execution_id)))
    }
}
