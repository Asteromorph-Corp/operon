use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::meta_storage::{
    MetaBackend, MetaClientApi, MetaConnApi, MetaStorageError, MetaTicketApi, MetaTxApi,
};
use crate::scheduler::events::{
    IndividualControlEvent, IndividualControlEventReceiver, InternalEvent, PeerEvent,
    PeerEventSenders, ServicePeerEventReceiver, ServicePeerEventSenderMap,
};
use crate::scheduler::queue::{AnyJobQueue, JobQueue};
use crate::scheduler::{SchedulerError, SpecWithMetadata, TaskSpec};
use crate::schema::{Job, SharedProgress, TaskMetadata, TaskState, Ticket, TicketStatus};
use crate::service::OperonService;
use crate::storage::OperonStorage;

/// # IndividualScheduler
///
/// IndividualScheduler is a scheduler for a single task.
/// It is responsible for:
///
/// * Keeping track of the tickets that are ready to run,
/// * Scheduling the jobs through a semaphore pool,
/// * Picking up job results and sending out `Event` messages, and
/// * Updating waiting tickets from `Event` messages.
///
/// The result a spawned worker reports back to its individual scheduler.
type WorkerResult<J, R, UErr, SErr, MErr> =
    Result<InternalEvent<J, R, UErr, SErr, MErr>, SchedulerError<UErr, SErr, MErr>>;

/// The set of workers an individual scheduler is currently awaiting.
type WorkerHandles<J, R, UErr, SErr, MErr> = JoinSet<WorkerResult<J, R, UErr, SErr, MErr>>;

/// Each individual scheduler conceptually "owns" a table in the ticket storage.
pub(super) struct IndividualScheduler<Svc, Sto, TS, MSto, const N: usize>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
    TS: TaskSpec<Svc, Sto, MSto>,
{
    pub spec: TS,
    pub meta: TaskMetadata<N>,
    pub service: Arc<Svc>,
    pub storage: Arc<Sto>,
    pub meta_storage: MSto,
    pub pool: Arc<Semaphore>,
    pub progress: SharedProgress,
    pub state: TaskState,
    pub handles: WorkerHandles<Job<N>, TS::Resolution, Svc::Error, Sto::Error, MSto::Error>,
}

impl<Svc, Sto, TS, MSto, const N: usize> IndividualScheduler<Svc, Sto, TS, MSto, N>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
    TS: TaskSpec<Svc, Sto, MSto, Job = Job<N>, Ticket = Ticket<N>>,
{
    pub(super) fn new(
        spec: SpecWithMetadata<Svc, Sto, TS, N>,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: MSto,
        pool_size: usize,
        progress: SharedProgress,
    ) -> Self {
        Self {
            spec: spec.spec,
            meta: spec.task_meta,
            storage,
            service,
            meta_storage,
            pool: Arc::new(Semaphore::new(pool_size)),
            progress,
            state: TaskState::Running,
            handles: JoinSet::new(),
        }
    }

    async fn set_state(&mut self, state: TaskState) {
        self.state = state;
        (*self.progress.write().await).set_state(state);
    }

    async fn update_progress(
        &mut self,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        let conn = self.meta_storage.scheduler_conn().await?;
        self.update_progress_with_client(conn.as_client()).await?;
        Ok(())
    }

    /// Call `update_state` with an ongoing connection.
    async fn update_progress_with_client(
        &mut self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        let (done, queued, waiting) = client.ticket(self.meta).get_status().await?;
        let finished = (*self.progress.write().await).update(done, queued, waiting);
        if finished && self.state != TaskState::Finished {
            self.state = TaskState::Finished;
            tracing::info!("All `{}` jobs are finished.", self.meta.id);
        }
        Ok(())
    }

    async fn initial_ready_tickets(&self) -> Result<Vec<Ticket<N>>, MetaStorageError<MSto::Error>> {
        let conn = self.meta_storage.scheduler_conn().await?;
        conn.as_client()
            .ticket(self.meta)
            .get_all(TicketStatus::Queued)
            .await
    }

    /// Handle a received event.
    ///
    /// One event corresponds to one metadata transaction.
    async fn on_event_ready_jobs(
        &mut self,
        event: PeerEvent<Svc::JobEnum, Svc::ResolutionEnum, Svc::TicketEnum>,
        peer_txs: &TS::PeerEventSenders,
    ) -> Result<Vec<Job<N>>, SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        let mut conn = self.meta_storage.scheduler_conn().await?;
        let tx = conn.transaction().await?;
        let ready_tickets = match event {
            PeerEvent::Job(job) => self.spec.on_receive_job(tx.as_client(), job).await?,
            PeerEvent::Resolution(resolution) => {
                self.spec
                    .on_receive_resolution(tx.as_client(), peer_txs, resolution)
                    .await?
            }
            PeerEvent::Explosion(explosion) => {
                self.spec
                    .on_receive_explosion(tx.as_client(), explosion)
                    .await?
            }
        };
        self.update_progress_with_client(tx.as_client()).await?;
        tx.commit().await?;
        let ready_jobs = ready_tickets
            .iter()
            .map(|t| t.resolve())
            .collect::<Option<Vec<_>>>()
            .ok_or(SchedulerError::other(
                "Some ready tickets are not actually ready to run",
            ))?;
        Ok(ready_jobs)
    }

    fn check_initial_data(&self, tickets: &[Ticket<N>]) -> bool {
        let mut all_ready = true;
        for ticket in tickets.iter().filter(|t| !t.is_ready()) {
            tracing::error!(
                "Restored ticket for `{}` job is not ready to run: {:?}",
                self.meta.id,
                ticket
            );
            all_ready = false;
        }
        all_ready
    }

    /// Drive the scheduler until every ticket of this task is finished
    /// **and** the broadcast channel has closed.
    ///
    /// Usually called by the top-level `Scheduler::run` with `tokio::spawn`.
    pub(super) async fn run(
        mut self,
        peer_tx_map: ServicePeerEventSenderMap<Svc>,
        peer_rx: ServicePeerEventReceiver<Svc>,
        ctrl_rx: IndividualControlEventReceiver,
        clean: bool,
    ) {
        let peer_txs = TS::PeerEventSenders::gather_from(peer_tx_map);

        let Ok(initial_tickets) = self.initial_ready_tickets().await else {
            self.set_state(TaskState::Error).await;
            return;
        };

        // Check if the initial data is valid, it can only be done if the task is not clean.
        if !clean && !self.check_initial_data(&initial_tickets) {
            self.set_state(TaskState::Error).await;
            return;
        }

        if let Err(e) = self.update_progress().await {
            tracing::error!("Failed to update UI state after initial data processing: {e}");
            self.set_state(TaskState::Error).await;
            return;
        }

        // Early return if the scheduler already finished (e.g. the last run completed this task).
        if self.state == TaskState::Finished {
            tracing::debug!(
                "Scheduler for `{}` exited due to being finished from the start.",
                self.meta.id
            );
            if let Err(e) = self.update_progress().await {
                tracing::error!("Failed to update UI state after scheduler run: {e}");
                self.set_state(TaskState::Error).await;
            }
            return;
        }
        let res = self
            .run_internal(initial_tickets, &peer_txs, peer_rx, ctrl_rx)
            .await;

        let id = &self.meta.id;
        if let Err(e) = &res {
            tracing::error!("Scheduler for `{id}` exited with an error: {e}");
            self.set_state(TaskState::Error).await;
        } else {
            match self.state {
                TaskState::Finished => tracing::debug!("Scheduler for `{id}` exited normally."),
                TaskState::Stopped => tracing::debug!("Scheduler for `{id}` was stopped."),
                _ => {
                    tracing::error!(
                        "Scheduler for `{id}` exited with an unexpected state: {:?}",
                        self.state,
                    );
                    self.set_state(TaskState::Error).await;
                }
            }
        }

        if let Err(e) = self.update_progress().await {
            tracing::error!("Failed to update UI state after scheduler run: {e}");
            self.set_state(TaskState::Error).await;
        }
    }

    async fn run_internal(
        &mut self,
        initial_tickets: Vec<Ticket<N>>,
        peer_txs: &TS::PeerEventSenders,
        mut peer_rx: ServicePeerEventReceiver<Svc>,
        mut ctrl_rx: IndividualControlEventReceiver,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        let pool = self.pool.clone();

        let initial_jobs = initial_tickets
            .iter()
            .map(|t| t.resolve())
            .collect::<Option<Vec<_>>>()
            .ok_or(SchedulerError::other(
                "Some initial tickets are not actually ready to run",
            ))?;
        let mut ready_jobs = AnyJobQueue::from_meta(initial_jobs, &self.meta);
        let mut got_all_peer_events = false;
        let mut is_gracefully_stopping = false;

        // Main event loop.
        loop {
            // Normal exit guard:
            // 1. `self.state == TaskState::Finished` only if all tickets are `Done`; this notably
            //    implies no more peer events are to be handled.
            // 2. `self.handles.is_empty()` only if all internal events are drained; i.e., all peer
            //    events to downstream schedulers have been sent.
            if self.state == TaskState::Finished && self.handles.is_empty() {
                return Ok(());
            }

            if is_gracefully_stopping && self.handles.is_empty() && got_all_peer_events {
                self.set_state(TaskState::Stopped).await;
                return Ok(());
            }

            ::tokio::select! {
                // 0. Check the control channel.
                Some(ctrl_event) = ctrl_rx.recv() => {
                    match ctrl_event {
                        IndividualControlEvent::Pause if self.state == TaskState::Running => self.handle_pause().await,
                        IndividualControlEvent::Resume if self.state == TaskState::Paused => self.handle_resume().await,
                        IndividualControlEvent::Quit { force: false } => {
                            self.handle_graceful_stop().await;
                            is_gracefully_stopping = true;
                        }
                        IndividualControlEvent::Quit { force: true } => {
                            tracing::info!("Aborting `{}` jobs.", self.meta.id);
                            self.handles.abort_all();
                            // If this is a finished scheduler rolling out peer events,
                            // don't change the state to `Stopped`,
                            // since it is already `Finished`.
                            if matches!(self.state, TaskState::Running | TaskState::Paused) {
                                self.set_state(TaskState::Stopped).await;
                            }
                            return Ok(());
                        }
                        _ => {}
                    }
                }

                // 1. An internal event.
                Some(int_event) = self.handles.join_next() => {
                    let int_event = int_event??;
                    self.update_progress().await?;
                    match int_event {
                        InternalEvent::JobSuccess(job, resolution) => {
                            tracing::trace!(
                                "{} received internal event: JobSuccess({job:?}, {resolution:?}).",
                                self.meta.id
                            );

                            self.spec.send_on_finish(peer_txs, job, resolution).await?;
                        }
                        InternalEvent::JobFailure(job, e) => {
                            tracing::error!("Job {job:?} failed: {e}");
                            return Err(e);
                        }
                    }
                }

                // 2. A peer event.
                event = peer_rx.recv(), if !got_all_peer_events => {
                    match event {
                        Some(evt) => {
                            tracing::trace!(
                                "{} received peer event: {evt:?}; \
                                Peer channel has {} events left.",
                                self.meta.id, peer_rx.len()
                            );
                            ready_jobs.extend(self.on_event_ready_jobs(evt, peer_txs).await?)
                        },
                        None => {
                            // The peer channel was closed,
                            // meaning that all peer updates were received,
                            // or that the upstream scheduler was gracefully stopped.
                            // Either way, we stop listening this branch.
                            tracing::debug!("`{}` finished receiving updates.", self.meta.id);
                            got_all_peer_events = true;
                        }
                    }
                }

                // 3. Run a job.
                // Gated on `Running`: this is what actually stops new jobs
                // from starting while paused/stopping.
                permit = pool.clone().acquire_owned(),
                    if !ready_jobs.is_empty() && self.state == TaskState::Running
                => {
                    let permit = permit?;
                    let job = ready_jobs.pop().ok_or(SchedulerError::other("Ready to run queue is empty"))?;
                    let task_id = self.meta.id;
                    let spec = self.spec.clone();
                    let storage = self.storage.clone();
                    let service = self.service.clone();
                    let meta_storage = self.meta_storage.clone();

                    // Move the permit into the task so it is released on drop.
                    self.handles.spawn(async move {
                        tracing::trace!("Running job {job:?} in `{task_id}` scheduler.");
                        let _permit = permit;
                        match spec.run_job(&*service, &*storage, meta_storage, job).await {
                            Ok(resolution) => {
                                tracing::trace!(
                                    "{task_id} worker exited with: JobSuccess({job:?}, {resolution:?}).",
                                );
                                Ok(InternalEvent::JobSuccess(job, resolution))
                            }
                            Err(e) => {
                                tracing::trace!(
                                    "{task_id} worker exited with: JobFailure({job:?}, {e:?});",
                                );
                                Ok(InternalEvent::JobFailure(job, e))
                            }
                        }
                    });
                }
            }
        }
    }

    async fn handle_pause(&mut self) {
        tracing::info!("Pausing `{}` jobs.", self.meta.id);
        self.set_state(TaskState::Paused).await;
    }

    async fn handle_resume(&mut self) {
        tracing::info!("Resuming `{}` jobs.", self.meta.id);
        self.set_state(TaskState::Running).await;
    }

    async fn handle_graceful_stop(&mut self) {
        if self.state == TaskState::Paused {
            return;
        }

        tracing::info!("Pausing `{}` jobs for graceful stop.", self.meta.id);
        self.set_state(TaskState::Paused).await;
    }
}
