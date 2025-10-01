use std::collections::HashMap;

use futures::{StreamExt, TryStreamExt};

use crate::meta_storage::{
    MetaClient, init_footprint, init_schema, init_ticket_status_type, init_ticket_summary,
};
use crate::operon::RunningState;
use crate::scheduler::{
    HandlerWithRx, HandlersWithChannels, JobHandler, JobRebuilder, PeerEvent, PeerEventSender,
    PrimaryHandler, SchedulerError,
};
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::{UiState, UiStateUpdate};

pub struct SchedulerHandler<Svc, Sto> {
    pub primary_handler: Box<dyn PrimaryHandler<Svc, Sto>>,
    pub job_handlers: Vec<Box<dyn JobHandler<Svc, Sto>>>,
}

impl<Svc, Sto> SchedulerHandler<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub fn new(
        primary_handler: Box<dyn PrimaryHandler<Svc, Sto>>,
        job_handlers: Vec<Box<dyn JobHandler<Svc, Sto>>>,
    ) -> Self {
        Self {
            primary_handler,
            job_handlers,
        }
    }

    pub(crate) fn prepare_channels(
        &self,
        channel_size: usize,
    ) -> HandlersWithChannels<'_, Svc, Sto> {
        let len = self.job_handlers.len();

        let mut schedules_with_rx = Vec::with_capacity(len);
        let mut peer_txs = HashMap::with_capacity(len);

        self.job_handlers.iter().for_each(|job_handler| {
            let (peer_tx, peer_rx) = tokio::sync::mpsc::channel::<
                PeerEvent<Svc::JobEnum, Svc::ResolutionEnum>,
            >(channel_size);
            peer_txs.insert(job_handler.job_id(), PeerEventSender::Up(peer_tx));
            schedules_with_rx.push(HandlerWithRx::new(job_handler.as_ref(), peer_rx));
        });

        HandlersWithChannels::new(schedules_with_rx, peer_txs)
    }

    pub(crate) async fn init_meta_storage(
        &self,
        client: MetaClient<'_>,
    ) -> Result<(), SchedulerError> {
        init_schema(client).await?;
        self.primary_handler.init_resolution(client).await?;
        for job_handler in &self.job_handlers {
            job_handler.init_resolution(client).await?;
        }
        init_ticket_summary(client).await?;
        init_ticket_status_type(client).await?;
        for job_handler in &self.job_handlers {
            job_handler.init_tickets(client).await?;
        }
        init_footprint(client).await?;

        Ok(())
    }

    /// Run a check on the data consistency between the data storage and the metadata storage.
    /// Return `true` if the data storage holds all needed data to restore, or `false` if it does
    /// not.
    ///
    /// This should be called only when the recovery state is either `AbortedUnchecked` or
    /// `GracefullyStopped`.
    pub(crate) async fn check_consistency(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
        primary_ub: usize,
    ) -> Result<bool, SchedulerError> {
        if !self
            .primary_handler
            .check_consistency(storage, client, primary_ub)
            .await?
        {
            return Ok(false);
        }
        for schedule in &self.job_handlers {
            if !schedule.check_consistency(storage, client).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub(crate) async fn clear_resolution(
        &self,
        client: MetaClient<'_>,
    ) -> Result<(), SchedulerError> {
        self.primary_handler.clear_resolution(client).await?;
        for spec in &self.job_handlers {
            spec.clear_resolution(client).await?;
        }
        Ok(())
    }

    pub(crate) async fn get_primary_resolution(
        &self,
        client: MetaClient<'_>,
    ) -> Result<Option<usize>, SchedulerError> {
        let primary_resolution = self.primary_handler.get_primary_resolution(client).await?;
        Ok(primary_resolution)
    }

    pub(crate) async fn put_primary_resolution(
        &self,
        client: MetaClient<'_>,
        primary_ub: usize,
    ) -> Result<(), SchedulerError> {
        self.primary_handler
            .put_primary_resolution(client, primary_ub)
            .await?;
        Ok(())
    }

    pub(crate) async fn clear_tickets(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        for schedule in &self.job_handlers {
            schedule.clear_tickets(client).await?;
        }
        Ok(())
    }

    pub(crate) async fn put_default_tickets(
        &self,
        client: MetaClient<'_>,
    ) -> Result<(), SchedulerError> {
        for schedule in &self.job_handlers {
            schedule.put_default_tickets(client).await?;
        }
        Ok(())
    }

    pub(crate) async fn update_ui(
        &self,
        client: MetaClient<'_>,
        ui_state: &mut UiState,
    ) -> Result<(), SchedulerError> {
        for schedule in &self.job_handlers {
            let (done, queued, waiting) = schedule.get_status(client).await?;
            let state = if queued + waiting == 0 {
                RunningState::Finished
            } else {
                RunningState::Running
            };

            ui_state.update_ui_state(UiStateUpdate::ProgressUpdate(
                schedule.job_id().to_string(),
                (done, queued, waiting, state, false),
            ))?;
        }
        Ok(())
    }

    pub(crate) async fn prepare_rebuilders(
        &self,
        storage: &Sto,
        client: MetaClient<'_>,
    ) -> Result<Vec<Box<dyn JobRebuilder>>, SchedulerError> {
        futures::stream::iter(&self.job_handlers)
            .then(|schedule| schedule.prepare_rebuild(storage, client))
            .try_collect::<Vec<_>>()
            .await
    }
}
