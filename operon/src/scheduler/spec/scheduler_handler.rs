use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use futures::{StreamExt, TryStreamExt};
use tokio::sync::RwLock;
use tokio::task::JoinSet;

use crate::meta_storage::{MetaBackend, MetaClientApi};
use crate::scheduler::events::{
    IndividualControlEventSender, PeerEvent, PeerEventSenderMap, ServicePeerEventReceiver,
    ServicePeerEventSenderMap,
};
use crate::scheduler::{SchedulerError, TaskHandler, TaskRebuilder};
use crate::schema::{CheckMode, Progress, SharedProgressMap, TableShape};
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub struct SchedulerHandler<Svc: OperonService, Sto: OperonStorage, MSto: MetaBackend> {
    pub task_handlers: Vec<Box<dyn TaskHandler<Svc, Sto, MSto>>>,
}

/// Helper struct for `Scheduler::prepare_channel`
///
/// An association of [`TaskHandler`] and event receiver channel
pub(crate) struct HandlerWithRx<'a, Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    pub handler: &'a dyn TaskHandler<Svc, Sto, MSto>,
    pub peer_rx: ServicePeerEventReceiver<Svc>,
}

/// Helper struct for `Scheduler::prepare_channel`
pub(crate) struct HandlersWithChannels<'a, Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    pub handlers_with_rx: Vec<HandlerWithRx<'a, Svc, Sto, MSto>>,
    pub peer_txs: PeerEventSenderMap<Svc::JobEnum, Svc::ResolutionEnum, Svc::TicketEnum>,
}

/// A control channel to a single `IndividualScheduler`, returned by
/// [`HandlersWithChannels::run_schedulers`].
pub(crate) struct ControlChannel {
    pub task_id: &'static str,
    pub upstream_tasks: Vec<&'static str>,
    pub tx: IndividualControlEventSender,
}

impl<Svc: OperonService, Sto: OperonStorage, MSto: MetaBackend> SchedulerHandler<Svc, Sto, MSto> {
    pub fn new(task_handlers: Vec<Box<dyn TaskHandler<Svc, Sto, MSto>>>) -> Self {
        Self { task_handlers }
    }

    /// Resolves every handler's pool size as the engine starts, so an environment variable that is
    /// missing or invalid panics before the user reaches the `run` command.
    pub(crate) fn validate_pool_sizes(&self) {
        for handler in &self.task_handlers {
            let _ = handler.pool_size();
        }
    }

    pub(crate) fn task_ids(&self) -> Vec<&'static str> {
        self.task_handlers
            .iter()
            .map(|task_handler| task_handler.task_id())
            .collect()
    }

    pub(crate) fn prepare_channels(
        &self,
        channel_size: usize,
    ) -> HandlersWithChannels<'_, Svc, Sto, MSto> {
        let len = self.task_handlers.len();

        let mut schedules_with_rx = Vec::with_capacity(len);
        let mut peer_txs = HashMap::with_capacity(len);

        self.task_handlers.iter().for_each(|task_handler| {
            let (peer_tx, peer_rx) = tokio::sync::mpsc::channel::<
                PeerEvent<Svc::JobEnum, Svc::ResolutionEnum, Svc::TicketEnum>,
            >(channel_size);
            peer_txs.insert(task_handler.task_id(), peer_tx);
            schedules_with_rx.push(HandlerWithRx::new(task_handler.as_ref(), peer_rx));
        });

        HandlersWithChannels::new(schedules_with_rx, peer_txs)
    }

    /// Initializes every metadata table the run needs.
    ///
    /// Returns [`STALE`](TableShape::STALE) if any task or dimension changed shape, discarding what
    /// its table held.
    pub(crate) async fn init_meta_storage(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<TableShape, SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        client.init_schema().await?;
        client.init_dimension_hash().await?;
        client.init_ticket_hash().await?;
        let resolution_is_stale = futures::stream::iter(&self.task_handlers)
            .then(|task_handler| async { task_handler.init_resolution(client).await })
            .try_fold(false, |was_stale, shape| async move {
                Ok(was_stale || shape.is_stale)
            })
            .await?;
        client.init_ticket_summary().await?;
        client.init_ticket_status_type().await?;
        let ticket_is_stale = futures::stream::iter(&self.task_handlers)
            .then(|task_handler| async { task_handler.init_tickets(client).await })
            .try_fold(false, |was_stale, shape| async move {
                Ok(was_stale || shape.is_stale)
            })
            .await?;
        client.init_footprint().await?;

        Ok(TableShape {
            is_stale: resolution_is_stale || ticket_is_stale,
        })
    }

    /// Run a check on the data consistency between the data storage and the metadata storage.
    /// Return the list of tasks that are found to be inconsistent.
    ///
    /// This should be called only when the stale state is either `Abort`, `GracefulStop`, or
    /// `Complete`.
    pub(crate) async fn check_consistency(
        &self,
        storage: &Sto,
        client: MSto::Client<'_>,
        mode: CheckMode,
    ) -> Result<Vec<&'static str>, SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        let mut inconsistent_tasks = Vec::new();
        for schedule in &self.task_handlers {
            if !schedule.check_consistency(storage, client, mode).await? {
                inconsistent_tasks.push(schedule.task_id());
                tracing::warn!(
                    "Consistency check failed for task handler: {}",
                    schedule.task_id()
                );
            } else {
                tracing::info!(
                    "Consistency check passed for task handler: {}",
                    schedule.task_id()
                );
            }
        }
        Ok(inconsistent_tasks)
    }

    pub(crate) async fn clear_resolution(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        for spec in &self.task_handlers {
            spec.clear_resolution(client).await?;
        }
        Ok(())
    }

    pub(crate) async fn clear_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        for schedule in &self.task_handlers {
            schedule.clear_tickets(client).await?;
        }
        Ok(())
    }

    pub(crate) async fn put_default_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        for schedule in &self.task_handlers {
            schedule.put_default_tickets(client).await?;
        }
        Ok(())
    }

    pub(crate) async fn update_ui(
        &self,
        progresses: &SharedProgressMap,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        for schedule in &self.task_handlers {
            let (done, queued, waiting) = schedule.get_status(client).await?;
            let Some(progress) = progresses.0.get(schedule.task_id()) else {
                return Err(SchedulerError::missing_progress(schedule.task_id()));
            };
            (*progress.write().await).update(done, queued, waiting);
        }
        Ok(())
    }

    pub(crate) async fn prepare_rebuilders(
        &self,
        storage: &Sto,
        progresses: &SharedProgressMap,
        client: MSto::Client<'_>,
        skip: &HashSet<String>,
    ) -> Result<
        Vec<Box<dyn TaskRebuilder<Svc, Sto, MSto>>>,
        SchedulerError<Svc::Error, Sto::Error, MSto::Error>,
    > {
        futures::stream::iter(self.task_handlers.iter().filter(|handler| {
            std::iter::once(handler.task_id())
                .chain(handler.all_upstream_tasks())
                .all(|task_id| !skip.contains(&task_id.to_string()))
        }))
        .then(|schedule| async {
            let Some(progress) = progresses.0.get(schedule.task_id()) else {
                return Err(SchedulerError::missing_progress(schedule.task_id()));
            };
            schedule
                .prepare_rebuild(storage, progress.clone(), client)
                .await
        })
        .try_collect::<Vec<_>>()
        .await
    }
}

impl<'a, Svc, Sto, MSto> HandlersWithChannels<'a, Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    pub fn new(
        handlers_with_rx: Vec<HandlerWithRx<'a, Svc, Sto, MSto>>,
        peer_txs: ServicePeerEventSenderMap<Svc>,
    ) -> Self {
        Self {
            handlers_with_rx,
            peer_txs,
        }
    }

    pub fn run_schedulers(
        self,
        service: &Arc<Svc>,
        storage: &Arc<Sto>,
        meta_storage: &MSto,
        progresses: &SharedProgressMap,
        clean: bool,
    ) -> (JoinSet<()>, Vec<ControlChannel>) {
        let mut futs = Vec::new();
        let mut channels = Vec::new();

        for HandlerWithRx { handler, peer_rx } in self.handlers_with_rx {
            let progress = progresses
                .0
                .get(handler.task_id())
                .cloned()
                .unwrap_or_else(|| Arc::new(RwLock::new(Progress::default())));
            let (ctrl_tx, ctrl_rx) = tokio::sync::mpsc::channel(64);
            futs.push(handler.run_scheduler(
                service.clone(),
                storage.clone(),
                meta_storage.clone(),
                progress,
                self.peer_txs.clone(),
                peer_rx,
                ctrl_rx,
                clean,
            ));
            channels.push(ControlChannel::new(
                handler.task_id(),
                handler.all_upstream_tasks(),
                ctrl_tx,
            ));
        }

        (JoinSet::from_iter(futs), channels)
    }
}

impl<'a, Svc, Sto, MSto> HandlerWithRx<'a, Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    pub fn new(
        handler: &'a dyn TaskHandler<Svc, Sto, MSto>,
        peer_rx: ServicePeerEventReceiver<Svc>,
    ) -> Self {
        Self { handler, peer_rx }
    }
}

impl ControlChannel {
    pub fn new(
        task_id: &'static str,
        upstream_tasks: Vec<&'static str>,
        tx: IndividualControlEventSender,
    ) -> Self {
        Self {
            task_id,
            upstream_tasks,
            tx,
        }
    }
}
