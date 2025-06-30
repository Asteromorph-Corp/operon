use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use tokio::sync::{RwLock, Semaphore};

use crate::{
    meta_storage::{MetaClient, MetaStorage},
    misc::Ticket,
    operon::RunningState,
    scheduler::{
        ControlEvent, ControlEventReceiver, IntEventReceiver, InternalEvent, JobManager, PeerEvent,
        PeerEventReceiver, PeerEventSender, PeerEventSenders, SchedulerError,
    },
    service::OperonService,
    storage::OperonStorage,
    ui::{UiState, UiStateUpdate},
};

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
pub struct IndividualScheduler<Svc, Sto, JM>
where
    Svc: OperonService,
    Sto: OperonStorage,
    JM: JobManager<Svc, Sto>,
{
    pub manager: JM,
    pub service: Arc<Svc>,
    pub storage: Arc<Sto>,
    pub meta_storage: MetaStorage,
    pub pool: Arc<Semaphore>,
    pub pool_size: usize,
    pub ui_state: Arc<RwLock<UiState>>,
    pub peer_txs: JM::PeerEventSenders,
    pub peer_rx: PeerEventReceiver<Svc::JobEnum, Svc::ResolutionEnum>,
    pub ctrl_rx: ControlEventReceiver,
}

impl<Svc, Sto, JM> IndividualScheduler<Svc, Sto, JM>
where
    Svc: OperonService,
    Sto: OperonStorage,
    JM: JobManager<Svc, Sto>,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        runner: JM,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: MetaStorage,
        pool_size: usize,
        ui_state: Arc<RwLock<UiState>>,
        mut peer_tx_map: HashMap<&'static str, PeerEventSender<Svc::JobEnum, Svc::ResolutionEnum>>,
        peer_rx: PeerEventReceiver<Svc::JobEnum, Svc::ResolutionEnum>,
        ctrl_rx: ControlEventReceiver,
    ) -> Self {
        // Create an internal channel for `InternalEvent`s.
        let peer_txs = JM::PeerEventSenders::gather_from(&mut peer_tx_map);

        // Downgrade unused peer senders to weak senders.
        for tx in peer_tx_map.values_mut() {
            tx.downgrade();
        }
        Self {
            manager: runner,
            storage,
            service,
            meta_storage,
            pool: Arc::new(Semaphore::new(pool_size)),
            pool_size,
            ui_state,
            peer_txs,
            peer_rx,
            ctrl_rx,
        }
    }

    async fn update_state(
        &self,
        client: MetaClient<'_>,
        returning: bool,
        state: &mut RunningState,
    ) -> Result<(), SchedulerError> {
        let (done, queued, waiting) = self.manager.get_status(client).await?;
        let state = if queued + waiting == 0 {
            if *state != RunningState::Finished {
                log::info!("All `{}` jobs are finished.", self.manager.job_id());
            }
            RunningState::Finished
        } else {
            *state
        };
        self.ui_state
            .write()
            .await
            .update_ui_state(UiStateUpdate::ProgressUpdate(
                self.manager.job_id().into(),
                (done, queued, waiting, state, returning),
            ))?;
        Ok(())
    }

    /// Call `update_ui` without an ongoing connection.
    async fn update_state_without_client(
        &self,
        returning: bool,
        state: &mut RunningState,
    ) -> Result<(), SchedulerError> {
        let conn = self.meta_storage.conn_static().await?;
        self.update_state(conn.as_client(), returning, state)
            .await?;
        Ok(())
    }

    /// Handle a received event.
    ///
    /// One event corresponds to one metadata transaction.
    async fn on_event_ready_tickets(
        &mut self,
        event: PeerEvent<Svc::JobEnum, Svc::ResolutionEnum>,
        state: &mut RunningState,
    ) -> Result<Vec<JM::Ticket>, SchedulerError> {
        let mut conn = self.meta_storage.conn_static().await?;
        let tx = conn.transaction().await?;
        let ready_tickets = match event {
            PeerEvent::Job(job) => {
                self.manager
                    .on_job_ready_tickets(tx.as_client(), job)
                    .await?
            }
            PeerEvent::Resolution(resolution) => {
                self.manager
                    .on_resolution_ready_tickets(tx.as_client(), resolution)
                    .await?
            }
        };
        self.update_state(tx.as_client(), false, state).await?;
        tx.commit().await?;
        Ok(ready_tickets)
    }

    fn check_initial_data(
        &self,
        initial_data: Vec<JM::Ticket>,
    ) -> Result<VecDeque<JM::Ticket>, ()> {
        let (ready, not_ready) = initial_data
            .into_iter()
            .partition::<Vec<_>, _>(|ticket| ticket.is_ready());

        if !not_ready.is_empty() {
            for ticket in not_ready {
                log::error!(
                    "Restored ticket for `{}` job is not ready to run: {ticket:?}",
                    self.manager.job_id()
                );
            }
            return Err(());
        }

        Ok(ready.into())
    }

    /// Drive the scheduler until every ticket of this job type is finished
    /// **and** the broadcast channel has closed.
    ///
    /// Usually called by the top-level `Scheduler::run` with `tokio::spawn`.
    pub async fn run(mut self, initial_data: Vec<JM::Ticket>) -> RunningState {
        let mut state = RunningState::Running;
        let Ok(ready_tickets) = self.check_initial_data(initial_data) else {
            // state = RunningState::Error;
            return RunningState::Error;
        };

        // Update the UI state before entering the loop.
        if self
            .update_state_without_client(false, &mut state)
            .await
            .is_err()
        {
            log::error!(
                "Failed to update UI state for `{}` scheduler after initial data processing.",
                self.manager.job_id()
            );
            // state = RunningState::Error;
            return RunningState::Error;
        }
        // Early return if the scheduler is already finished (e.g. the last run completed this job).
        if state == RunningState::Finished {
            log::debug!(
                "Scheduler for `{}` exited due to being finished from the start.",
                self.manager.job_id()
            );
            self.update_state_without_client(true, &mut state)
                .await
                .unwrap_or_else(|e| {
                    log::error!("Failed to update UI state after scheduler run: {e}");
                    state = RunningState::Error
                });
            return state;
        }
        let res = self.run_internal(ready_tickets, &mut state).await;

        // Close peer senders
        self.peer_txs.downgrade_all();

        match (&res, state) {
            (Ok(()), RunningState::Finished) => {
                log::debug!("Scheduler for `{}` exited normally.", self.manager.job_id())
            }
            (Ok(()), RunningState::Stopped) => {
                log::debug!(
                    "Scheduler for `{}` was stopped and exited.",
                    self.manager.job_id()
                )
            }
            (Ok(()), _) => {
                log::error!(
                    "Scheduler for `{}` exited with an unexpected state: {state:?}",
                    self.manager.job_id(),
                );
                state = RunningState::Error;
            }
            (Err(e), _) => {
                log::error!(
                    "Scheduler for `{}` exited with an error: {e}",
                    self.manager.job_id()
                );
                state = RunningState::Error;
            }
        }

        // Update the UI state one last time.
        self.update_state_without_client(true, &mut state)
            .await
            .unwrap_or_else(|e| {
                log::error!("Failed to update UI state after scheduler run: {e}");
                state = RunningState::Error;
            });
        state
    }

    async fn run_internal(
        &mut self,
        mut ready_tickets: VecDeque<JM::Ticket>,
        state: &mut RunningState,
    ) -> Result<(), SchedulerError> {
        let pool = self.pool.clone();
        let mut got_all_updates = false;

        let (int_tx, mut int_rx) =
            tokio::sync::mpsc::unbounded_channel::<InternalEvent<JM::Job, JM::Resolution>>();

        // Main event loop.
        loop {
            // This loop cannot be entered with the `Error` or `Stopped` state,
            // as changing the state to `Error` or `Stopped` always exits `run_internal` immediately.
            // 0. Check the control channel.
            let ctrl_event = self.ctrl_rx.borrow_and_update().clone();
            match ctrl_event {
                ControlEvent::Pause { targets, cascade } => {
                    self.handle_pause(targets, cascade, state).await?
                }
                ControlEvent::Resume { targets } => self.handle_resume(targets, state).await?,
                ControlEvent::GracefulStop => {
                    if self
                        .handle_graceful_stop(state, got_all_updates, &int_rx)
                        .await?
                    {
                        return Ok(());
                    }
                }
                ControlEvent::Abort => {
                    log::info!("Aborting `{}` jobs.", self.manager.job_id());
                    // If this is a finished scheduler rolling out peer events,
                    // don't change the state to `Stopped`,
                    // since it is already `Finished`.
                    if matches!(state, RunningState::Running | RunningState::Paused) {
                        *state = RunningState::Stopped;
                    }
                    return Ok(());
                }
                _ => (), // No-op for other control events.
            }

            ::tokio::select! {
                // 0. Also wait on the control channel to avoid being stuck in this loop.
                ctrl_event = self.ctrl_rx.changed() => {
                    ctrl_event?;
                }

                // 1. An internal event.
                int_event = int_rx.recv() => {
                    let int_event = int_event.ok_or(
                        SchedulerError::Other(
                            "Scheduler internal channel closed prematurely".into()
                        )
                    )?;
                    self.update_state_without_client(false, state).await?;
                    match int_event {
                        InternalEvent::JobSuccess(job, resolution) => {
                            // Trace the job success
                            log::trace!(
                                "{} received internal event: JobSuccess({job:?}, {resolution:?}); \
                                Internal channel has {} events left.",
                                self.manager.job_id(), int_rx.len()
                            );

                            // Broadcast the job result events
                            self.manager.send_event(&self.peer_txs, job, resolution).await?;
                            // If all the tickets are finished
                            // AND the scheduler's internal events are drained,
                            // exit the loop.
                            if *state == RunningState::Finished && int_rx.is_empty(){
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
                event = self.peer_rx.recv(), if !got_all_updates => {
                    match event {
                        Some(evt) => {
                            // Trace the peer event
                            log::trace!(
                                "{} received peer event: {evt:?}; \
                                Peer channel has {} events left.",
                                self.manager.job_id(), self.peer_rx.len()
                            );
                            ready_tickets.extend(self.on_event_ready_tickets(evt, state).await?)
                        },
                        None => {
                            // The peer channel was closed,
                            // meaning that all peer updates were received,
                            // or that the upstream scheduler was gracefully stopped.
                            // Either way, we stop listening this branch.
                            log::debug!("`{}` finished receiving updates.", self.manager.job_id());
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
                    let ticket = ready_tickets.pop_front().ok_or(SchedulerError::Other("Ready to run queue is empty".into()))?;
                    let job = ticket.resolve().ok_or(SchedulerError::Other("Ticket is not ready to run".into()))?;
                    let job_id = self.manager.job_id();
                    let runner = self.manager.clone();
                    let storage = self.storage.clone();
                    let service = self.service.clone();
                    let meta_storage = self.meta_storage.clone();
                    let int_sender = int_tx.clone();

                    // Move the permit into the task so it is released on drop.
                    // The metadata storage operations are grouped in one transaction here.
                    tokio::spawn(async move {
                        // Trace the job start.
                        log::trace!("Running job {job:?} in `{job_id}` scheduler.");
                        let _permit = permit;
                        let mut conn = meta_storage.conn().await?;
                        let tx = conn.transaction().await?;
                        match runner.run_job(&*service, &*storage, tx.as_client(), &job).await {
                            Ok(resolution) => {
                                // Mark the ticket as done in the ticket storage
                                runner.mark_done(tx.as_client(), &job).await?;
                                runner.put_resolution(tx.as_client(), &resolution).await?;
                                tx.commit().await?;

                                // Alert the results to the scheduler
                                int_sender.send(InternalEvent::JobSuccess(job.clone(), resolution.clone()))
                                    .map_err(|e| SchedulerError::Other(format!("Failed to send internal event: {e}")))?;
                                log::trace!(
                                    "{job_id} sent internal event: JobSuccess({job:?}, {resolution:?});",
                                );

                                Ok(())
                            }
                            Err(e) => {
                                // Rollback the transaction
                                tx.rollback().await?;

                                // Alert the error to the scheduler
                                int_sender.send(InternalEvent::JobFailure(job, e))
                                    .map_err(|e| SchedulerError::Other(format!("Failed to send internal event: {e}")))?;
                                Err(SchedulerError::Other("Job failed".into()))
                            }
                        }
                    });
                }
            }
        }
    }

    async fn handle_pause(
        &mut self,
        targets: Vec<String>,
        cascade: bool,
        state: &mut RunningState,
    ) -> Result<(), SchedulerError> {
        if !(targets.is_empty()
            || targets.iter().any(|t| t == self.manager.job_id())
            || cascade && targets.iter().any(|t| JM::is_descendant_of(t)))
        {
            return Ok(());
        }

        match state {
            RunningState::Running => {
                log::info!("Pausing `{}` jobs.", self.manager.job_id());
                *state = RunningState::Paused;
                self.update_state_without_client(false, state).await?;
                // Acquire and forget all permits.
                let permit = self
                    .pool
                    .clone()
                    .acquire_many_owned(self.pool_size as u32)
                    .await?;
                permit.forget();
                log::debug!("Remaining `{}` jobs were finished.", self.manager.job_id());
            }
            RunningState::Paused => (),   // Silent no-op, already paused.
            RunningState::Finished => (), // No jobs to pause, no-op.
            // This scheduler is executing the leftover `send_on_finish` events,
            // which are not affected by the pause.
            _ => {
                log::error!(
                    "Scheduler for `{}` entered event loop in an unexpected state: {:?}",
                    self.manager.job_id(),
                    state
                );
                *state = RunningState::Error;
                return Err(SchedulerError::Other(
                    "Scheduler entered event loop in an unexpected state".into(),
                ));
            }
        }
        Ok(())
    }

    async fn handle_resume(
        &mut self,
        targets: Vec<String>,
        state: &mut RunningState,
    ) -> Result<(), SchedulerError> {
        if !(targets.is_empty() || targets.iter().any(|t| t == self.manager.job_id())) {
            return Ok(());
        }

        match state {
            RunningState::Paused => {
                log::info!("Resuming `{}` jobs.", self.manager.job_id());
                *state = RunningState::Running;
                self.update_state_without_client(false, state).await?;
                // Add back all permits.
                self.pool.add_permits(self.pool_size);
            }
            RunningState::Running => (), // Silent no-op, already running.
            RunningState::Finished => (), // No jobs to resume, no-op.
            _ => {
                log::error!(
                    "Scheduler for `{}` entered event loop in an unexpected state: {:?}",
                    self.manager.job_id(),
                    state
                );
                *state = RunningState::Error;
                return Err(SchedulerError::Other(
                    "Scheduler entered event loop in an unexpected state".into(),
                ));
            }
        }
        Ok(())
    }

    async fn handle_graceful_stop(
        &mut self,
        state: &mut RunningState,
        got_all_updates: bool,
        int_rx: &IntEventReceiver<JM::Job, JM::Resolution>,
    ) -> Result<bool, SchedulerError> {
        if *state == RunningState::Running {
            log::info!(
                "Pausing `{}` jobs for graceful stop.",
                self.manager.job_id()
            );
            *state = RunningState::Paused;
            self.update_state_without_client(false, state).await?;
            // Acquire and forget all permits.
            let permit = self
                .pool
                .clone()
                .acquire_many_owned(self.pool_size as u32)
                .await?;
            permit.forget();
            log::debug!("Remaining `{}` jobs were finished.", self.manager.job_id());
        }
        match state {
            RunningState::Paused | RunningState::Finished => {
                if int_rx.is_empty() && got_all_updates {
                    log::info!("Gracefully stopped `{}` jobs.", self.manager.job_id());
                    // If the state is `Paused`, set it to `Stopped`,
                    // If the state is `Finished`, keep it as `Finished`.
                    if *state == RunningState::Paused {
                        *state = RunningState::Stopped;
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            _ => {
                log::error!(
                    "Scheduler for `{}` entered event loop in an unexpected state: {:?}",
                    self.manager.job_id(),
                    state
                );
                *state = RunningState::Error;
                Err(SchedulerError::Other(
                    "Scheduler entered event loop in an unexpected state".into(),
                ))
            }
        }
    }
}
