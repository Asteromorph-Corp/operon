use std::collections::HashSet;

use async_trait::async_trait;
use tokio::time::Instant;
use uuid::Uuid;

use crate::meta_storage::{MemMetaStorage, MetaBackend, MetaConnApi, MetaTxApi};
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
    ) -> TransitionState<SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        TransitionState::new(Self::new(ctx, channel_size, run_id, skip))
    }

    fn into_start(self) -> TransitionState<SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
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
    type Error = SchedulerError<Svc::Error, Sto::Error, MSto::Error>;

    fn warn_msg(&self) -> Option<&'static str> {
        Some("Rebuild in progress, commands will be handled after rebuild completes.")
    }

    async fn execute(self) -> Result<NextState<Self::Error>, Self::Error> {
        let start = Instant::now();

        let scratch = MemMetaStorage::default();
        let scratch_handler = self.ctx.handler.to_mem();

        let scratch_conn = scratch
            .scheduler_conn()
            .await
            .map_err(SchedulerError::from_mem_meta)?;
        let scratch_client = scratch_conn.as_client();

        // The in-memory tables are registered here, before anything writes to them.
        scratch_handler
            .init_meta_storage(scratch_client)
            .await
            .map_err(SchedulerError::from_mem)?;

        let mut conn = self.ctx.meta_storage.scheduler_conn().await?;
        self.ctx
            .handler
            .hydrate_mem(conn.as_client(), scratch_client)
            .await?;

        let rebuilders = scratch_handler
            .prepare_rebuilders(
                &self.ctx.storage,
                &self.ctx.progresses,
                scratch_client,
                &self.skip,
            )
            .await
            .map_err(SchedulerError::from_mem)?;

        scratch_handler
            .clear_resolution(scratch_client)
            .await
            .map_err(SchedulerError::from_mem)?;
        scratch_handler
            .clear_tickets(scratch_client)
            .await
            .map_err(SchedulerError::from_mem)?;

        scratch_handler
            .put_default_tickets(scratch_client)
            .await
            .map_err(SchedulerError::from_mem)?;
        scratch_handler
            .update_ui(&self.ctx.progresses, scratch_client)
            .await
            .map_err(SchedulerError::from_mem)?;

        for rebuilder in rebuilders {
            rebuilder
                .rebuild(scratch_client)
                .await
                .map_err(SchedulerError::from_mem)?;
        }

        let tx = conn.transaction().await?;
        self.ctx
            .handler
            .dump_mem(scratch_client, tx.as_client())
            .await?;
        tx.commit().await?;

        tracing::info!(
            "Rebuild completed in: {:?}, starting the run.",
            start.elapsed()
        );

        Ok(NextState::next(self.into_start()))
    }
}
