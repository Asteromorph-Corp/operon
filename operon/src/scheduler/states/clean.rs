use async_trait::async_trait;
use uuid::Uuid;

use crate::meta_storage::{MetaBackend, MetaClientApi, MetaConnApi, MetaTxApi};
use crate::scheduler::SchedulerError;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::states::start::StartTransition;
use crate::scheduler::states::{NextState, SchedulerTransition, TransitionState};
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub struct CleanTransition<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    ctx: SchedulerContext<Svc, Sto, MSto>,
    channel_size: usize,
    run_id: Uuid,
}

impl<Svc, Sto, MSto> CleanTransition<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    pub fn new(ctx: SchedulerContext<Svc, Sto, MSto>, channel_size: usize, run_id: Uuid) -> Self {
        Self {
            ctx,
            channel_size,
            run_id,
        }
    }

    pub fn state(
        ctx: SchedulerContext<Svc, Sto, MSto>,
        channel_size: usize,
        run_id: Uuid,
    ) -> TransitionState<MSto::Error> {
        TransitionState::new(Self::new(ctx, channel_size, run_id))
    }

    fn into_start(self) -> TransitionState<MSto::Error> {
        StartTransition::state(self.ctx, self.channel_size, self.run_id, true)
    }
}

#[async_trait]
impl<Svc, Sto, MSto> SchedulerTransition<MSto::Error> for CleanTransition<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    fn warn_msg(&self) -> Option<&'static str> {
        None
    }

    async fn execute(self) -> Result<NextState<MSto::Error>, SchedulerError<MSto::Error>> {
        let mut conn = self.ctx.meta_storage.scheduler_conn().await?;
        let tx = conn.transaction().await?;

        self.ctx.handler.clear_resolution(tx.as_client()).await?;
        self.ctx.handler.clear_tickets(tx.as_client()).await?;
        tx.as_client().clear_footprint().await?;

        self.ctx.handler.put_default_tickets(tx.as_client()).await?;

        tx.commit().await?;

        Ok(NextState::next(self.into_start()))
    }
}
