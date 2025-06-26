use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use async_trait::async_trait;
use tokio::sync::{RwLock, Semaphore};

use crate::{
    dimension::Ticket,
    meta_storage::{MetaClient, MetaStorage, MetaStorageError},
    operon::RunningState,
    scheduler::{
        ControlEvent, ControlEventReceiver, IntEventReceiver, InternalEvent, JobRunner,
        PeerEventReceiver, PeerEventSender, PeerEventSenders, SchedulerError,
    },
    service::OperonService,
    storage::OperonStorage,
    ui::UiState,
};

/// # IndividualSchedulerOps trait
///
/// This trait defines the event operations that an individual scheduler must implement.
#[async_trait]
pub trait IndividualSchedulerOps<T>: Send + Sync + 'static
where
    T: Ticket,
{
    /// Handle a job event.
    async fn poll_ready(
        &mut self,
        conn: MetaClient<'_>,
        ready_tickets: &mut VecDeque<T>,
    ) -> Result<(), SchedulerError>;

    /// Send out the events associated with the just processed job.
    ///
    /// NOTE that peer receivers might be dropped due to a stop signal or an error,
    /// so this function should not error out if the receiver is gone.
    async fn send_event(&self) -> Result<(), SchedulerError>;

    /// Update the UI state ticket counts associated with this scheduler.
    /// Called whenever the UI state information may be changed.
    async fn update_ui(
        &mut self,
        conn: MetaClient<'_>,
        returning: bool,
    ) -> Result<(), SchedulerError>;
}

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
pub struct IndividualScheduler<Svc, Sto, MSto, T>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaStorage,
    T: Ticket,
{
    pub runner: Box<dyn JobRunner<Svc, Sto, T::Job, T::Resolution>>,
    pub service: Arc<Svc>,
    pub storage: Arc<Sto>,
    pub meta_storage: Arc<MSto>,
    pub pool: Arc<Semaphore>,
    pub pool_size: usize,
    pub ui_state: Arc<RwLock<UiState>>,
    pub peer_txs: T::PeerEventSenders,
    pub peer_rx: PeerEventReceiver,
    pub ctrl_rx: ControlEventReceiver,
}

impl<Svc, Sto, MSto, T> IndividualScheduler<Svc, Sto, MSto, T>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaStorage,
    T: Ticket,
    Self: IndividualSchedulerOps<T>,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        runner: Box<dyn JobRunner<Svc, Sto, T::Job, T::Resolution>>,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: Arc<MSto>,
        pool_size: usize,
        ui_state: Arc<RwLock<UiState>>,
        mut peer_tx_map: HashMap<String, PeerEventSender>,
        peer_rx: PeerEventReceiver,
        ctrl_rx: ControlEventReceiver,
    ) -> Self {
        // Create an internal channel for `InternalEvent`s.
        let peer_txs = T::PeerEventSenders::gather_from(&mut peer_tx_map);

        // Downgrade unused peer senders to weak senders.
        for tx in peer_tx_map.values_mut() {
            tx.downgrade();
        }
        Self {
            runner,
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

    /// Call `update_ui` without an ongoing connection.
    async fn update_ui_no_conn(&mut self, returning: bool) -> Result<(), SchedulerError> {
        let client = self.meta_storage.client().await?;
        self.update_ui(MetaClient::Object(&client), returning)
            .await?;
        Ok(())
    }

    /// Handle a received event.
    ///
    /// One event corresponds to one metadata transaction.
    async fn on_receive_peer_event(
        &mut self,
        ready_tickets: &mut VecDeque<T>,
    ) -> Result<(), SchedulerError> {
        let mut client = self.meta_storage.client().await?;
        let tx = client.transaction().await.map_err(MetaStorageError::from)?;
        let conn = MetaClient::Transaction(&tx);
        self.poll_ready(conn, ready_tickets).await?;
        self.update_ui(conn, false).await?;
        tx.commit().await.map_err(MetaStorageError::from)?;
        Ok(())
    }

    fn check_initial_data(initial_data: Vec<T>) -> Result<VecDeque<T>, ()> {
        let (ready, not_ready) = initial_data
            .into_iter()
            .partition::<Vec<_>, _>(|ticket| ticket.is_ready());

        if !not_ready.is_empty() {
            for ticket in not_ready {
                log::error!(
                    "Restored ticket for `{}` job is not ready to run: {ticket:?}",
                    T::job_type()
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
    pub async fn run(mut self, initial_data: Vec<T>) -> RunningState {
        let mut state = RunningState::Running;
        let Ok(ready_tickets) = Self::check_initial_data(initial_data) else {
            // state = RunningState::Error;
            return RunningState::Error;
        };

        // Update the UI state before entering the loop.
        if self.update_ui_no_conn(false).await.is_err() {
            log::error!(
                "Failed to update UI state for `{}` scheduler after initial data processing.",
                T::job_type()
            );
            // state = RunningState::Error;
            return RunningState::Error;
        }
        // Early return if the scheduler is already finished (e.g. the last run completed this job).
        if state == RunningState::Finished {
            log::debug!(
                "Scheduler for `{}` exited due to being finished from the start.",
                T::job_type()
            );
            self.update_ui_no_conn(true).await.unwrap_or_else(|e| {
                log::error!("Failed to update UI state after scheduler run: {e}");
                state = RunningState::Error;
            });
            return state;
        }
        let res = self.run_internal(ready_tickets, &mut state).await;

        // Close peer senders
        self.peer_txs.downgrade_all();

        match (&res, state) {
            (Ok(()), RunningState::Finished) => {
                log::debug!("Scheduler for `{}` exited normally.", T::job_type())
            }
            (Ok(()), RunningState::Stopped) => {
                log::debug!("Scheduler for `{}` was stopped and exited.", T::job_type())
            }
            (Ok(()), _) => {
                log::error!(
                    "Scheduler for `{}` exited with an unexpected state: {state:?}",
                    T::job_type(),
                );
                state = RunningState::Error;
            }
            (Err(e), _) => {
                log::error!(
                    "Scheduler for `{}` exited with an error: {e}",
                    T::job_type()
                );
                state = RunningState::Error;
            }
        }

        // Update the UI state one last time.
        self.update_ui_no_conn(true).await.unwrap_or_else(|e| {
            log::error!("Failed to update UI state after scheduler run: {e}");
            state = RunningState::Error;
        });
        state
    }

    async fn run_internal(
        &mut self,
        mut ready_tickets: VecDeque<T>,
        state: &mut RunningState,
    ) -> Result<(), SchedulerError> {
        let pool = self.pool.clone();
        let mut got_all_updates = false;

        let (int_tx, mut int_rx) =
            tokio::sync::mpsc::unbounded_channel::<InternalEvent<T::Job, T::Resolution>>();

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
                    log::info!("Aborting `{}` jobs.", T::job_type());
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
                    self.update_ui_no_conn(false).await?;
                    match int_event {
                        InternalEvent::JobSuccess(job, resolution) => {
                            // Trace the job success
                            log::trace!(
                                "{} received internal event: JobSuccess({job:?}, {resolution:?}); \
                                Internal channel has {} events left.",
                                T::job_type(), int_rx.len()
                            );

                            // Broadcast the job result events
                            self.send_event().await?;
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
                        Some(_) => {
                            // Trace the peer event
                            log::trace!(
                                "{} received peer event: {event:?}; \
                                Peer channel has {} events left.",
                                T::job_type(), self.peer_rx.len()
                            );
                            self.on_receive_peer_event(&mut ready_tickets).await?
                        },
                        None => {
                            // The peer channel was closed,
                            // meaning that all peer updates were received,
                            // or that the upstream scheduler was gracefully stopped.
                            // Either way, we stop listening this branch.
                            log::debug!("`{}` finished receiving updates.", T::job_type());
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
                    let runner = self.runner.clone_box();
                    let storage = self.storage.clone();
                    let service = self.service.clone();
                    let meta_storage = self.meta_storage.clone();
                    let int_sender = int_tx.clone();

                    // Move the permit into the task so it is released on drop.
                    // The metadata storage operations are grouped in one transaction here.
                    tokio::spawn(async move {
                        // Trace the job start.
                        log::trace!("Running job {job:?} in `{}` scheduler.", T::job_type());
                        let _permit = permit;
                        let mut client = meta_storage.client().await?;
                        let tx = client.transaction().await.map_err(MetaStorageError::from)?;
                        let conn = MetaClient::Transaction(&tx);
                        match runner.run_job(conn, &*service, &*storage, &job).await {
                            Ok(resolution) => {
                                // Mark the ticket as done in the ticket storage
                                runner.mark_done(conn, &job).await?;
                                runner.put_resolution(conn, &resolution).await?;
                                tx.commit().await.map_err(MetaStorageError::from)?;

                                // Alert the results to the scheduler
                                int_sender.send(InternalEvent::JobSuccess(job.clone(), resolution.clone()))
                                    .map_err(|e| SchedulerError::Other(format!("Failed to send internal event: {e}")))?;
                                log::trace!(
                                    "{} sent internal event: JobSuccess({job:?}, {resolution:?});",
                                    T::job_type()
                                );

                                Ok(())
                            }
                            Err(e) => {
                                // Rollback the transaction
                                tx.rollback().await.map_err(MetaStorageError::from)?;

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
            || targets.contains(&T::job_type())
            || cascade && targets.iter().any(|t| T::is_descendant_of(t)))
        {
            return Ok(());
        }

        match state {
            RunningState::Running => {
                log::info!("Pausing `{}` jobs.", T::job_type());
                *state = RunningState::Paused;
                self.update_ui_no_conn(false).await?;
                // Acquire and forget all permits.
                let permit = self
                    .pool
                    .clone()
                    .acquire_many_owned(self.pool_size as u32)
                    .await?;
                permit.forget();
                log::debug!("Remaining `{}` jobs were finished.", T::job_type());
            }
            RunningState::Paused => (),   // Silent no-op, already paused.
            RunningState::Finished => (), // No jobs to pause, no-op.
            // This scheduler is executing the leftover `send_on_finish` events,
            // which are not affected by the pause.
            _ => {
                log::error!(
                    "Scheduler for `{}` entered event loop in an unexpected state: {:?}",
                    T::job_type(),
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
        if !(targets.is_empty() || targets.contains(&T::job_type())) {
            return Ok(());
        }

        match state {
            RunningState::Paused => {
                log::info!("Resuming `{}` jobs.", T::job_type());
                *state = RunningState::Running;
                self.update_ui_no_conn(false).await?;
                // Add back all permits.
                self.pool.add_permits(self.pool_size);
            }
            RunningState::Running => (), // Silent no-op, already running.
            RunningState::Finished => (), // No jobs to resume, no-op.
            _ => {
                log::error!(
                    "Scheduler for `{}` entered event loop in an unexpected state: {:?}",
                    T::job_type(),
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
        int_rx: &IntEventReceiver<T::Job, T::Resolution>,
    ) -> Result<bool, SchedulerError> {
        if *state == RunningState::Running {
            log::info!("Pausing `{}` jobs for graceful stop.", T::job_type());
            *state = RunningState::Paused;
            self.update_ui_no_conn(false).await?;
            // Acquire and forget all permits.
            let permit = self
                .pool
                .clone()
                .acquire_many_owned(self.pool_size as u32)
                .await?;
            permit.forget();
            log::debug!("Remaining `{}` jobs were finished.", T::job_type());
        }
        match state {
            RunningState::Paused | RunningState::Finished => {
                if int_rx.is_empty() && got_all_updates {
                    log::info!("Gracefully stopped `{}` jobs.", T::job_type());
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
                    T::job_type(),
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
