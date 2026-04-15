use async_trait::async_trait;
use tokio::time::Instant;
use uuid::Uuid;

use crate::scheduler::SchedulerError;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::states::start::StartTransition;
use crate::scheduler::states::{NextState, SchedulerTransition, TransitionState};
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub struct RebuildTransition<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    ctx: SchedulerContext<Svc, Sto>,
    channel_size: usize,
    run_id: Uuid,
}

impl<Svc, Sto> RebuildTransition<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn new(ctx: SchedulerContext<Svc, Sto>, channel_size: usize, run_id: Uuid) -> Self {
        Self {
            ctx,
            channel_size,
            run_id,
        }
    }

    pub fn state(
        ctx: SchedulerContext<Svc, Sto>,
        channel_size: usize,
        run_id: Uuid,
    ) -> TransitionState {
        TransitionState::new(Self::new(ctx, channel_size, run_id))
    }

    fn into_start(self) -> TransitionState {
        StartTransition::state(self.ctx, self.channel_size, self.run_id, false)
    }
}

#[async_trait]
impl<Svc, Sto> SchedulerTransition for RebuildTransition<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    fn warn_msg(&self) -> Option<&'static str> {
        Some("Rebuild in progress, commands will be handled after rebuild completes.")
    }

    async fn execute(self) -> Result<NextState, SchedulerError> {
        let start = Instant::now();

        let mut conn = self.ctx.meta_storage.ui_conn().await?;
        let tx = conn.transaction().await?;

        let rebuilders = self
            .ctx
            .handler
            .prepare_rebuilders(&self.ctx.storage, &self.ctx.progresses, tx.as_client())
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

        Ok(NextState::from(self.into_start()))
    }
}
