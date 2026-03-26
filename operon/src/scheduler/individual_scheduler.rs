use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::meta_storage::{MetaClient, MetaStorage, MetaStorageError};
use crate::scheduler::{
    IndividualControlEvent, IndividualControlEventReceiver, InternalEvent, JobSpec, PeerEvent,
    PeerEventSenders, SchedulerError, ServicePeerEventReceiver, ServicePeerEventSenderMap,
    SpecWithMetadata,
};
use crate::schema::{Job, JobMetadata, SharedProgress, TaskState, Ticket, TicketStatus};
use crate::service::OperonService;
use crate::storage::OperonStorage;

/// # IndividualScheduler
///
/// IndividualScheduler is a scheduler for a single job type.
/// It is responsible for:
///
/// * Keeping track of the tickets that are ready to run,
/// * Scheduling the jobs through a semaphore pool,
/// * Picking up job results and sending out `Event` messages, and
/// * Updating waiting tickets from `Event` messages.
///
/// Each individual scheduler conceptually "owns" a table in the ticket storage.
pub struct IndividualScheduler<Svc, Sto, JS, const N: usize>
where
    Svc: OperonService,
    Sto: OperonStorage,
    JS: JobSpec<Svc, Sto>,
{
    pub spec: JS,
    pub meta: JobMetadata<N>,
    pub service: Arc<Svc>,
    pub storage: Arc<Sto>,
    pub meta_storage: MetaStorage,
    pub pool: Arc<Semaphore>,
    pub pool_size: usize,
    pub progress: SharedProgress,
    pub state: TaskState,
    pub handles: JoinSet<Result<InternalEvent<Job<N>, JS::Resolution>, SchedulerError>>,
}

impl<Svc, Sto, JS, const N: usize> IndividualScheduler<Svc, Sto, JS, N>
where
    Svc: OperonService,
    Sto: OperonStorage,
    JS: JobSpec<Svc, Sto, Job = Job<N>, Ticket = Ticket<N>>,
{
    pub fn new(
        spec: SpecWithMetadata<Svc, Sto, JS, N>,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: MetaStorage,
        pool_size: usize,
        progress: SharedProgress,
    ) -> Self {
        Self {
            spec: spec.spec,
            meta: spec.job_meta,
            storage,
            service,
            meta_storage,
            pool: Arc::new(Semaphore::new(pool_size)),
            pool_size,
            progress,
            state: TaskState::Running,
            handles: JoinSet::new(),
        }
    }

    async fn set_state(&mut self, state: TaskState) {
        self.state = state;
        (*self.progress.write().await).set_state(state);
    }

    async fn update_progress(&mut self) -> Result<(), SchedulerError> {
        let conn = self.meta_storage.ui_conn().await?;
        self.update_progress_with_client(conn.as_client()).await?;
        Ok(())
    }

    /// Call `update_state` with an ongoing connection.
    async fn update_progress_with_client(
        &mut self,
        client: MetaClient<'_>,
    ) -> Result<(), SchedulerError> {
        let (done, queued, waiting) = client.ticket(self.meta).get_status().await?;
        let finished = (*self.progress.write().await).update(done, queued, waiting);
        if finished {
            self.state = TaskState::Finished;
            tracing::info!("All `{}` jobs are finished.", self.meta.id);
        }
        Ok(())
    }

    async fn initial_ready_tickets(&self) -> Result<Vec<Ticket<N>>, MetaStorageError> {
        let conn = self.meta_storage.conn().await?;
        conn.as_client()
            .ticket(self.meta)
            .get_all(TicketStatus::Queued)
            .await
    }

    /// Handle a received event.
    ///
    /// One event corresponds to one metadata transaction.
    async fn on_event_ready_tickets(
        &mut self,
        event: PeerEvent<Svc::JobEnum, Svc::ResolutionEnum, Svc::TicketEnum>,
        peer_txs: &JS::PeerEventSenders,
    ) -> Result<Vec<Ticket<N>>, SchedulerError> {
        let mut conn = self.meta_storage.conn_static().await?;
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
        Ok(ready_tickets)
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

    /// Drive the scheduler until every ticket of this job type is finished
    /// **and** the broadcast channel has closed.
    ///
    /// Usually called by the top-level `Scheduler::run` with `tokio::spawn`.
    pub async fn run(
        mut self,
        peer_tx_map: ServicePeerEventSenderMap<Svc>,
        peer_rx: ServicePeerEventReceiver<Svc>,
        ctrl_rx: IndividualControlEventReceiver,
        clean: bool,
    ) {
        // Create an internal channel for `InternalEvent`s.
        let peer_txs = JS::PeerEventSenders::gather_from(peer_tx_map);

        let Ok(initial_tickets) = self.initial_ready_tickets().await else {
            self.set_state(TaskState::Error).await;
            return;
        };

        // Check if the initial data is valid, it can only be done if the job is not clean.
        if !clean && !self.check_initial_data(&initial_tickets) {
            self.set_state(TaskState::Error).await;
            return;
        }

        // Update the UI state before entering the loop.
        if let Err(e) = self.update_progress().await {
            tracing::error!("Failed to update UI state after initial data processing: {e}");
            self.set_state(TaskState::Error).await;
            return;
        }

        // Early return if the scheduler is already finished (e.g. the last run completed this job).
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

        // Update the UI state one last time.
        if let Err(e) = self.update_progress().await {
            tracing::error!("Failed to update UI state after scheduler run: {e}");
            self.set_state(TaskState::Error).await;
        }
    }

    async fn run_internal(
        &mut self,
        initial_tickets: Vec<Ticket<N>>,
        peer_txs: &JS::PeerEventSenders,
        mut peer_rx: ServicePeerEventReceiver<Svc>,
        mut ctrl_rx: IndividualControlEventReceiver,
    ) -> Result<(), SchedulerError> {
        let pool = self.pool.clone();

        let mut ready_tickets: VecDeque<Ticket<N>> = initial_tickets.into();
        let mut got_all_updates = false;
        let mut is_stopping = false;

        // Main event loop.
        loop {
            if is_stopping && self.handles.is_empty() && got_all_updates {
                self.set_state(TaskState::Stopped).await;
                return Ok(());
            }

            ::tokio::select! {
                // 0. Check the control channel.
                Some(ctrl_event) = ctrl_rx.recv() => {
                    match ctrl_event {
                        IndividualControlEvent::Pause if self.state == TaskState::Running => self.handle_pause().await?,
                        IndividualControlEvent::Resume if self.state == TaskState::Paused => self.handle_resume().await?,
                        IndividualControlEvent::Quit { force: false } => {
                            self.handle_graceful_stop().await?;
                            is_stopping = true;
                        }
                        IndividualControlEvent::Quit { force: true } => {
                            tracing::info!("Aborting `{}` jobs.", self.meta.id);
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
                            // Trace the job success
                            tracing::trace!(
                                "{} received internal event: JobSuccess({job:?}, {resolution:?}).",
                                self.meta.id
                            );

                            // Broadcast the job result events
                            self.spec.send_on_finish(peer_txs, job, resolution).await?;
                            // If all the tickets are finished
                            // AND the scheduler's internal events are drained,
                            // exit the loop.
                            if self.state == TaskState::Finished && self.handles.is_empty() {
                                return Ok(());
                            }
                        }
                        InternalEvent::JobFailure(job, e) => {
                            // Log the error
                            tracing::error!("Job {job:?} failed: {e}");
                            // Return the error to the top-level scheduler
                            return Err(e);
                        }
                    }
                }

                // 2. A peer event.
                event = peer_rx.recv(), if !got_all_updates => {
                    match event {
                        Some(evt) => {
                            // Trace the peer event
                            tracing::trace!(
                                "{} received peer event: {evt:?}; \
                                Peer channel has {} events left.",
                                self.meta.id, peer_rx.len()
                            );
                            ready_tickets.extend(self.on_event_ready_tickets(evt, peer_txs).await?)
                        },
                        None => {
                            // The peer channel was closed,
                            // meaning that all peer updates were received,
                            // or that the upstream scheduler was gracefully stopped.
                            // Either way, we stop listening this branch.
                            tracing::debug!("`{}` finished receiving updates.", self.meta.id);
                            got_all_updates = true;
                        }
                    }
                }

                // 3. Run a job.
                // If the scheduler is paused, the pool will not yield a permit
                // since the pool will have forgotten the permits.
                permit = pool.clone().acquire_owned(),
                    if !ready_tickets.is_empty()
                => {
                    let permit = permit?;
                    let ticket = ready_tickets.pop_front().ok_or(SchedulerError::other("Ready to run queue is empty"))?;
                    let job = ticket.resolve().ok_or(SchedulerError::other("Ticket is not ready to run"))?;
                    let job_id = self.meta.id;
                    let spec = self.spec.clone();
                    let job_meta = self.meta;
                    let storage = self.storage.clone();
                    let service = self.service.clone();
                    let meta_storage = self.meta_storage.clone();
                    // let int_sender = int_tx.clone();

                    // Move the permit into the task so it is released on drop.
                    // The metadata storage operations are grouped in one transaction here.
                    self.handles.spawn(async move {
                        // Trace the job start.
                        tracing::trace!("Running job {job:?} in `{job_id}` scheduler.");
                        let _permit = permit;
                        let mut conn = meta_storage.conn().await?;
                        let tx = conn.transaction().await?;
                        match spec.run_job(&*service, &*storage, tx.as_client(), job).await {
                            Ok(resolution) => {
                                // Mark the ticket as done in the ticket storage
                                tx.as_client().ticket(job_meta).mark_done(job).await?;
                                tx.commit().await?;

                                // Alert the results to the scheduler
                                tracing::trace!(
                                    "{job_id} worker exited with: JobSuccess({job:?}, {resolution:?}).",
                                );
                                Ok(InternalEvent::JobSuccess(job, resolution))
                            }
                            Err(e) => {
                                // Rollback the transaction
                                tx.rollback().await?;

                                // Alert the error to the scheduler
                                tracing::trace!(
                                    "{job_id} worker exited with: JobFailure({job:?}, {e:?});",
                                );
                                Ok(InternalEvent::JobFailure(job, e))
                            }
                        }
                    });
                }
            }
        }
    }

    async fn handle_pause(&mut self) -> Result<(), SchedulerError> {
        tracing::info!("Pausing `{}` jobs.", self.meta.id);
        self.set_state(TaskState::Paused).await;
        // Acquire and forget all permits.
        let permit = self
            .pool
            .clone()
            .acquire_many_owned(self.pool_size as u32)
            .await?;
        permit.forget();
        tracing::debug!("Remaining `{}` jobs were finished.", self.meta.id);
        Ok(())
    }

    async fn handle_resume(&mut self) -> Result<(), SchedulerError> {
        tracing::info!("Resuming `{}` jobs.", self.meta.id);
        self.set_state(TaskState::Running).await;
        // Add back all permits.
        self.pool.add_permits(self.pool_size);
        Ok(())
    }

    async fn handle_graceful_stop(&mut self) -> Result<(), SchedulerError> {
        if self.state == TaskState::Paused {
            return Ok(());
        }

        tracing::info!("Pausing `{}` jobs for graceful stop.", self.meta.id);
        self.set_state(TaskState::Paused).await;

        // Acquire and forget all permits.
        let permit = self
            .pool
            .clone()
            .acquire_many_owned(self.pool_size as u32)
            .await?;
        permit.forget();
        tracing::debug!("Remaining `{}` jobs were finished.", self.meta.id);
        Ok(())
    }
}
