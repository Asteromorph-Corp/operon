use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::meta_storage::{MetaClient, MetaStorage, MetaStorageError};
use crate::scheduler::{
    ExecutionState, IndividualControlEvent, IndividualControlEventReceiver, InternalEvent, JobSpec,
    PeerEvent, PeerEventSenders, SchedulerError, ServicePeerEventReceiver,
    ServicePeerEventSenderMap, SpecWithMetadata,
};
use crate::schema::{Job, JobMetadata, Progress, SharedProgress, Ticket, TicketStatus};
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
    pub all_upstream_jobs: Vec<&'static str>,
    pub service: Arc<Svc>,
    pub storage: Arc<Sto>,
    pub meta_storage: MetaStorage,
    pub pool: Arc<Semaphore>,
    pub pool_size: usize,
    pub progress: SharedProgress,
    pub handles: JoinSet<Result<InternalEvent<Job<N>, JS::Resolution>, SchedulerError>>,
}

impl<Svc, Sto, JS, const N: usize> IndividualScheduler<Svc, Sto, JS, N>
where
    Svc: OperonService,
    Sto: OperonStorage,
    JS: JobSpec<Svc, Sto, Job = Job<N>, Ticket = Ticket<N>>,
{
    #[allow(clippy::too_many_arguments)]
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
            all_upstream_jobs: spec.all_upstream_jobs,
            storage,
            service,
            meta_storage,
            pool: Arc::new(Semaphore::new(pool_size)),
            pool_size,
            progress,
            handles: JoinSet::new(),
        }
    }

    async fn update_state(
        &self,
        client: MetaClient<'_>,
        state: &mut ExecutionState,
    ) -> Result<(), SchedulerError> {
        let (done, queued, waiting) = client.ticket(self.meta).get_status().await?;
        if queued + waiting == 0 && *state != ExecutionState::Finished {
            log::info!("All `{}` jobs are finished.", self.meta.id);
            *state = ExecutionState::Finished
        }
        *self.progress.write().await = Progress::new(done, queued, waiting, *state);
        Ok(())
    }

    /// Call `update_ui` without an ongoing connection.
    async fn update_state_without_client(
        &self,
        state: &mut ExecutionState,
    ) -> Result<(), SchedulerError> {
        let conn = self.meta_storage.ui_conn().await?;
        self.update_state(conn.as_client(), state).await?;
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
        state: &mut ExecutionState,
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
        self.update_state(tx.as_client(), state).await?;
        tx.commit().await?;
        Ok(ready_tickets)
    }

    fn check_initial_data(&self, tickets: &[Ticket<N>]) -> bool {
        let mut all_ready = true;
        for ticket in tickets.iter().filter(|t| !t.is_ready()) {
            log::error!(
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
    ) -> ExecutionState {
        // Create an internal channel for `InternalEvent`s.
        let peer_txs = JS::PeerEventSenders::gather_from(peer_tx_map);

        let mut state = ExecutionState::Running;

        let Ok(initial_tickets) = self.initial_ready_tickets().await else {
            return ExecutionState::Error;
        };

        // Check if the initial data is valid, it can only be done if the job is not clean.
        if !clean && !self.check_initial_data(&initial_tickets) {
            return ExecutionState::Error;
        }

        // Update the UI state before entering the loop.
        if self.update_state_without_client(&mut state).await.is_err() {
            log::error!(
                "Failed to update UI state for `{}` scheduler after initial data processing.",
                self.meta.id
            );
            // state = RunningState::Error;
            return ExecutionState::Error;
        }
        // Early return if the scheduler is already finished (e.g. the last run completed this job).
        if state == ExecutionState::Finished {
            log::debug!(
                "Scheduler for `{}` exited due to being finished from the start.",
                self.meta.id
            );
            self.update_state_without_client(&mut state)
                .await
                .unwrap_or_else(|e| {
                    log::error!("Failed to update UI state after scheduler run: {e}");
                    state = ExecutionState::Error
                });
            return state;
        }
        let res = self
            .run_internal(initial_tickets, &peer_txs, peer_rx, ctrl_rx, &mut state)
            .await;

        match (&res, state) {
            (Ok(()), ExecutionState::Finished) => {
                log::debug!("Scheduler for `{}` exited normally.", self.meta.id)
            }
            (Ok(()), ExecutionState::Stopped) => {
                log::debug!("Scheduler for `{}` was stopped and exited.", self.meta.id)
            }
            (Ok(()), _) => {
                log::error!(
                    "Scheduler for `{}` exited with an unexpected state: {state:?}",
                    self.meta.id,
                );
                state = ExecutionState::Error;
            }
            (Err(e), _) => {
                log::error!("Scheduler for `{}` exited with an error: {e}", self.meta.id);
                state = ExecutionState::Error;
            }
        }

        // Update the UI state one last time.
        self.update_state_without_client(&mut state)
            .await
            .unwrap_or_else(|e| {
                log::error!("Failed to update UI state after scheduler run: {e}");
                state = ExecutionState::Error;
            });
        state
    }

    async fn run_internal(
        &mut self,
        initial_tickets: Vec<Ticket<N>>,
        peer_txs: &JS::PeerEventSenders,
        mut peer_rx: ServicePeerEventReceiver<Svc>,
        mut ctrl_rx: IndividualControlEventReceiver,
        state: &mut ExecutionState,
    ) -> Result<(), SchedulerError> {
        let pool = self.pool.clone();

        let mut ready_tickets: VecDeque<Ticket<N>> = initial_tickets.into();
        let mut got_all_updates = false;
        let mut is_stopping = false;

        // Main event loop.
        loop {
            if is_stopping && self.handles.is_empty() && got_all_updates {
                *state = ExecutionState::Stopped;
                return Ok(());
            }

            ::tokio::select! {
                // 0. Check the control channel.
                Some(ctrl_event) = ctrl_rx.recv() => {
                    match ctrl_event {
                        IndividualControlEvent::Pause if *state == ExecutionState::Running => self.handle_pause(state).await?,
                        IndividualControlEvent::Resume if *state == ExecutionState::Paused => self.handle_resume(state).await?,
                        IndividualControlEvent::Quit { force: false } => {
                            self.handle_graceful_stop(state).await?;
                            is_stopping = true;
                        }
                        IndividualControlEvent::Quit { force: true } => {
                            log::info!("Aborting `{}` jobs.", self.meta.id);
                            // If this is a finished scheduler rolling out peer events,
                            // don't change the state to `Stopped`,
                            // since it is already `Finished`.
                            if matches!(state, ExecutionState::Running | ExecutionState::Paused) {
                                *state = ExecutionState::Stopped;
                            }
                            return Ok(());
                        }
                        _ => {}
                    }
                }

                // 1. An internal event.
                Some(int_event) = self.handles.join_next() => {
                    let int_event = int_event??;
                    self.update_state_without_client( state).await?;
                    match int_event {
                        InternalEvent::JobSuccess(job, resolution) => {
                            // Trace the job success
                            log::trace!(
                                "{} received internal event: JobSuccess({job:?}, {resolution:?}).",
                                self.meta.id
                            );

                            // Broadcast the job result events
                            self.spec.send_on_finish(peer_txs, job, resolution).await?;
                            // If all the tickets are finished
                            // AND the scheduler's internal events are drained,
                            // exit the loop.
                            if *state == ExecutionState::Finished && self.handles.is_empty() {
                                return Ok(());
                            }
                        }
                        InternalEvent::JobFailure(job, e) => {
                            // Log the error
                            log::error!("Job {job:?} failed: {e}");
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
                            log::trace!(
                                "{} received peer event: {evt:?}; \
                                Peer channel has {} events left.",
                                self.meta.id, peer_rx.len()
                            );
                            ready_tickets.extend(self.on_event_ready_tickets(evt, state, peer_txs).await?)
                        },
                        None => {
                            // The peer channel was closed,
                            // meaning that all peer updates were received,
                            // or that the upstream scheduler was gracefully stopped.
                            // Either way, we stop listening this branch.
                            log::debug!("`{}` finished receiving updates.", self.meta.id);
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
                        log::trace!("Running job {job:?} in `{job_id}` scheduler.");
                        let _permit = permit;
                        let mut conn = meta_storage.conn().await?;
                        let tx = conn.transaction().await?;
                        match spec.run_job(&*service, &*storage, tx.as_client(), job).await {
                            Ok(resolution) => {
                                // Mark the ticket as done in the ticket storage
                                tx.as_client().ticket(job_meta).mark_done(job).await?;
                                tx.commit().await?;

                                // Alert the results to the scheduler
                                log::trace!(
                                    "{job_id} worker exited with: JobSuccess({job:?}, {resolution:?}).",
                                );
                                Ok(InternalEvent::JobSuccess(job, resolution))
                            }
                            Err(e) => {
                                // Rollback the transaction
                                tx.rollback().await?;

                                // Alert the error to the scheduler
                                log::trace!(
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

    async fn handle_pause(&mut self, state: &mut ExecutionState) -> Result<(), SchedulerError> {
        log::info!("Pausing `{}` jobs.", self.meta.id);
        *state = ExecutionState::Paused;
        self.update_state_without_client(state).await?;
        // Acquire and forget all permits.
        let permit = self
            .pool
            .clone()
            .acquire_many_owned(self.pool_size as u32)
            .await?;
        permit.forget();
        log::debug!("Remaining `{}` jobs were finished.", self.meta.id);
        Ok(())
    }

    async fn handle_resume(&mut self, state: &mut ExecutionState) -> Result<(), SchedulerError> {
        log::info!("Resuming `{}` jobs.", self.meta.id);
        *state = ExecutionState::Running;
        self.update_state_without_client(state).await?;
        // Add back all permits.
        self.pool.add_permits(self.pool_size);
        Ok(())
    }

    async fn handle_graceful_stop(
        &mut self,
        state: &mut ExecutionState,
    ) -> Result<(), SchedulerError> {
        if *state == ExecutionState::Paused {
            return Ok(());
        }

        log::info!("Pausing `{}` jobs for graceful stop.", self.meta.id);
        *state = ExecutionState::Paused;
        self.update_state_without_client(state).await?;
        // Acquire and forget all permits.
        let permit = self
            .pool
            .clone()
            .acquire_many_owned(self.pool_size as u32)
            .await?;
        permit.forget();
        log::debug!("Remaining `{}` jobs were finished.", self.meta.id);
        Ok(())
    }
}
