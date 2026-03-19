use std::collections::HashMap;

use futures::{StreamExt, TryStreamExt};

use crate::meta_storage::MetaClient;
use crate::scheduler::{
    ExecutionState, HandlerWithRx, HandlersWithChannels, JobHandler, JobRebuilder, PeerEvent,
    SchedulerError,
};
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::{UiState, UiStateUpdate};

pub struct SchedulerHandler<Svc: OperonService, Sto: OperonStorage> {
    pub job_handlers: Vec<Box<dyn JobHandler<Svc, Sto>>>,
}

impl<Svc: OperonService, Sto: OperonStorage> SchedulerHandler<Svc, Sto> {
    pub fn new(job_handlers: Vec<Box<dyn JobHandler<Svc, Sto>>>) -> Self {
        Self { job_handlers }
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
                PeerEvent<Svc::JobEnum, Svc::ResolutionEnum, Svc::TicketEnum>,
            >(channel_size);
            peer_txs.insert(job_handler.job_id(), peer_tx);
            schedules_with_rx.push(HandlerWithRx::new(job_handler.as_ref(), peer_rx));
        });

        HandlersWithChannels::new(schedules_with_rx, peer_txs)
    }

    pub(crate) async fn init_meta_storage(
        &self,
        client: MetaClient<'_>,
    ) -> Result<(), SchedulerError> {
        client.init_schema().await?;
        client.init_dimension_hash().await?;
        client.init_ticket_hash().await?;
        for job_handler in &self.job_handlers {
            job_handler.init_resolution(client).await?;
        }
        client.init_ticket_summary().await?;
        client.init_ticket_status_type().await?;
        for job_handler in &self.job_handlers {
            job_handler.init_tickets(client).await?;
        }
        client.init_footprint().await?;

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
        mode: crate::ui::CheckMode,
    ) -> Result<bool, SchedulerError> {
        for schedule in &self.job_handlers {
            if !schedule.check_consistency(storage, client, mode).await? {
                return Ok(false);
            }
            log::info!(
                "Consistency check passed for job handler: {}",
                schedule.job_id()
            );
        }
        Ok(true)
    }

    pub(crate) async fn clear_resolution(
        &self,
        client: MetaClient<'_>,
    ) -> Result<(), SchedulerError> {
        for spec in &self.job_handlers {
            spec.clear_resolution(client).await?;
        }
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
                ExecutionState::Finished
            } else {
                ExecutionState::Running
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
