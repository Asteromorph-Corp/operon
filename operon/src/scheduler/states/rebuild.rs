use std::collections::HashSet;

use async_trait::async_trait;
use tokio::time::Instant;
use uuid::Uuid;

use crate::meta_storage::{MetaBackend, MetaConnApi, MetaTxApi};
use crate::scheduler::SchedulerError;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::states::start::StartTransition;
use crate::scheduler::states::{NextState, SchedulerTransition, TransitionState};
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub struct RebuildTransition<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    ctx: SchedulerContext<Svc, Sto, MSto>,
    channel_size: usize,
    run_id: Uuid,
    skip: HashSet<String>,
}

impl<Svc, Sto, MSto> RebuildTransition<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    pub fn new(
        ctx: SchedulerContext<Svc, Sto, MSto>,
        channel_size: usize,
        run_id: Uuid,
        skip: HashSet<String>,
    ) -> Self {
        Self {
            ctx,
            channel_size,
            run_id,
            skip,
        }
    }

    pub fn state(
        ctx: SchedulerContext<Svc, Sto, MSto>,
        channel_size: usize,
        run_id: Uuid,
        skip: HashSet<String>,
    ) -> TransitionState<SchedulerError<MSto::Error, Sto::Error, Svc::Error>> {
        TransitionState::new(Self::new(ctx, channel_size, run_id, skip))
    }

    fn into_start(self) -> TransitionState<SchedulerError<MSto::Error, Sto::Error, Svc::Error>> {
        StartTransition::state(self.ctx, self.channel_size, self.run_id, false)
    }
}

#[async_trait]
impl<Svc, Sto, MSto> SchedulerTransition for RebuildTransition<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    type Error = SchedulerError<MSto::Error, Sto::Error, Svc::Error>;

    fn warn_msg(&self) -> Option<&'static str> {
        Some("Rebuild in progress, commands will be handled after rebuild completes.")
    }

    async fn execute(self) -> Result<NextState<Self::Error>, Self::Error> {
        let start = Instant::now();

        let mut conn = self.ctx.meta_storage.scheduler_conn().await?;
        let tx = conn.transaction().await?;

        let rebuilders = self
            .ctx
            .handler
            .prepare_rebuilders(
                &self.ctx.storage,
                &self.ctx.progresses,
                tx.as_client(),
                &self.skip,
            )
            .await?;

        self.ctx.handler.clear_resolution(tx.as_client()).await?;
        self.ctx.handler.clear_tickets(tx.as_client()).await?;

        self.ctx.handler.put_default_tickets(tx.as_client()).await?;
        self.ctx
            .handler
            .update_ui(&self.ctx.progresses, tx.as_client())
            .await?;

        for rebuilder in rebuilders {
            rebuilder.rebuild(tx.as_client()).await?;
        }

        tx.commit().await?;
        tracing::info!(
            "Rebuild completed in: {:?}, starting the run.",
            start.elapsed()
        );

        Ok(NextState::next(self.into_start()))
    }
}
