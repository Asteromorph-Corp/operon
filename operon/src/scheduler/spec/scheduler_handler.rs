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
use crate::scheduler::{JobHandler, JobRebuilder, SchedulerError};
use crate::schema::{CheckMode, Progress, SharedProgressMap, TableShape};
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub struct SchedulerHandler<Svc: OperonService, Sto: OperonStorage, MSto: MetaBackend> {
    pub job_handlers: Vec<Box<dyn JobHandler<Svc, Sto, MSto>>>,
}

/// Helper struct for `Scheduler::prepare_channel`
///
/// An association of `JobManager` and event receiver channel
pub(crate) struct HandlerWithRx<'a, Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    pub handler: &'a dyn JobHandler<Svc, Sto, MSto>,
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
    pub job_id: &'static str,
    pub upstream_jobs: Vec<&'static str>,
    pub tx: IndividualControlEventSender,
}

impl<Svc: OperonService, Sto: OperonStorage, MSto: MetaBackend> SchedulerHandler<Svc, Sto, MSto> {
    pub fn new(job_handlers: Vec<Box<dyn JobHandler<Svc, Sto, MSto>>>) -> Self {
        Self { job_handlers }
    }

    /// Eagerly resolve every handler's pool size, panicking immediately if an
    /// environment variable is missing or invalid rather than waiting until
    /// the user issues a `run` command.
    pub(crate) fn validate_pool_sizes(&self) {
        for handler in &self.job_handlers {
            let _ = handler.pool_size();
        }
    }

    pub(crate) fn job_ids(&self) -> Vec<&'static str> {
        self.job_handlers
            .iter()
            .map(|job_handler| job_handler.job_id())
            .collect()
    }

    pub(crate) fn prepare_channels(
        &self,
        channel_size: usize,
    ) -> HandlersWithChannels<'_, Svc, Sto, MSto> {
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

    /// Initializes every metadata table the run needs.
    ///
    /// Returns [`Stale`](TableShape::Stale) if any job or dimension changed shape, discarding what
    /// its table held.
    pub(crate) async fn init_meta_storage(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<TableShape, SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        client.init_schema().await?;
        client.init_dimension_hash().await?;
        client.init_ticket_hash().await?;
        let resolution_shape = futures::stream::iter(&self.job_handlers)
            .then(|job_handler| async { job_handler.init_resolution(client).await })
            .try_fold(TableShape::Current, |shape, job_shape| async move {
                Ok(shape.merge(job_shape))
            })
            .await?;
        client.init_ticket_summary().await?;
        client.init_ticket_status_type().await?;
        let ticket_shape = futures::stream::iter(&self.job_handlers)
            .then(|job_handler| async { job_handler.init_tickets(client).await })
            .try_fold(TableShape::Current, |shape, job_shape| async move {
                Ok(shape.merge(job_shape))
            })
            .await?;
        client.init_footprint().await?;

        Ok(TableShape::merge(resolution_shape, ticket_shape))
    }

    /// Run a check on the data consistency between the data storage and the metadata storage.
    /// Return the list of jobs that are found to be inconsistent.
    ///
    /// This should be called only when the stale state is either `Abort`, `GracefulStop`, or
    /// `Complete`.
    pub(crate) async fn check_consistency(
        &self,
        storage: &Sto,
        client: MSto::Client<'_>,
        mode: CheckMode,
    ) -> Result<Vec<&'static str>, SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        let mut inconsistent_jobs = Vec::new();
        for schedule in &self.job_handlers {
            if !schedule.check_consistency(storage, client, mode).await? {
                inconsistent_jobs.push(schedule.job_id());
                tracing::warn!(
                    "Consistency check failed for job handler: {}",
                    schedule.job_id()
                );
            } else {
                tracing::info!(
                    "Consistency check passed for job handler: {}",
                    schedule.job_id()
                );
            }
        }
        Ok(inconsistent_jobs)
    }

    pub(crate) async fn clear_resolution(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        for spec in &self.job_handlers {
            spec.clear_resolution(client).await?;
        }
        Ok(())
    }

    pub(crate) async fn clear_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        for schedule in &self.job_handlers {
            schedule.clear_tickets(client).await?;
        }
        Ok(())
    }

    pub(crate) async fn put_default_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        for schedule in &self.job_handlers {
            schedule.put_default_tickets(client).await?;
        }
        Ok(())
    }

    pub(crate) async fn update_ui(
        &self,
        progresses: &SharedProgressMap,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        for schedule in &self.job_handlers {
            let (done, queued, waiting) = schedule.get_status(client).await?;
            let Some(progress) = progresses.0.get(schedule.job_id()) else {
                return Err(SchedulerError::missing_progress(schedule.job_id()));
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
        Vec<Box<dyn JobRebuilder<Svc, Sto, MSto>>>,
        SchedulerError<Svc::Error, Sto::Error, MSto::Error>,
    > {
        futures::stream::iter(self.job_handlers.iter().filter(|handler| {
            std::iter::once(handler.job_id())
                .chain(handler.all_upstream_jobs())
                .all(|job_id| !skip.contains(&job_id.to_string()))
        }))
        .then(|schedule| async {
            let Some(progress) = progresses.0.get(schedule.job_id()) else {
                return Err(SchedulerError::missing_progress(schedule.job_id()));
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
                .get(handler.job_id())
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
                handler.job_id(),
                handler.all_upstream_jobs(),
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
        handler: &'a dyn JobHandler<Svc, Sto, MSto>,
        peer_rx: ServicePeerEventReceiver<Svc>,
    ) -> Self {
        Self { handler, peer_rx }
    }
}

impl ControlChannel {
    pub fn new(
        job_id: &'static str,
        upstream_jobs: Vec<&'static str>,
        tx: IndividualControlEventSender,
    ) -> Self {
        Self {
            job_id,
            upstream_jobs,
            tx,
        }
    }
}
