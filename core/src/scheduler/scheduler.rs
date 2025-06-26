use std::{collections::HashMap, sync::Arc};

use futures::future::try_join_all;
use tokio::{sync::RwLock, task::JoinSet};

use crate::{
    meta_storage::{MetaClient, MetaContext, MetaStorage, MetaStorageError},
    operon::RunningState,
    scheduler::{
        ControlEvent, ControlEventReceiver, IndividualSchedule, PeerEvent, PeerEventSender,
        RecoveryState, RecoveryStateSender, RunMode, SchedulerError, SchedulerOptions,
        misc::{PreparedSchedules, ScheduleWithRx},
    },
    service::OperonService,
    storage::OperonStorage,
    ui::UiState,
};

/// # Scheduler
///
/// The orchestrating scheduler that manages the individual schedulers.
///
/// It is responsible for:
///
/// * Initialization of the metadata storage,
/// * initialization of the individual schedulers, and
/// * communication between the UI and the individual schedulers.
pub struct Scheduler<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaStorage,
{
    service: Arc<Svc>,
    storage: Arc<Sto>,
    meta_storage: Arc<MSto>,
    schedules: Vec<Box<dyn IndividualSchedule<Svc, Sto, MSto>>>,
    ui_state: Arc<RwLock<UiState>>,
    ctrl_rx: ControlEventReceiver,
    rec_tx: RecoveryStateSender,
    internal_channel_size: usize,
}

impl<Svc, Sto, MSto> Scheduler<Svc, Sto, MSto>
where
    Sto: OperonStorage,
    Svc: OperonService,
    MSto: MetaStorage,
{
    /// Initialize a new scheduler and its associated storages.
    pub async fn new(
        service: Arc<Svc>,
        storage: Arc<Sto>,
        schedules: Vec<Box<dyn IndividualSchedule<Svc, Sto, MSto>>>,
        ui_state: Arc<RwLock<UiState>>,
        ctrl_rx: ControlEventReceiver,
        rec_tx: RecoveryStateSender,
        options: SchedulerOptions,
    ) -> Result<Self, SchedulerError> {
        let (internal_channel_size, meta_storage_options) = options.split();
        let meta_storage = Arc::new(MSto::new(MetaContext::new(meta_storage_options).await?));
        meta_storage.init().await?;

        Ok(Self {
            service,
            storage,
            meta_storage,
            schedules,
            ui_state,
            ctrl_rx,
            rec_tx,
            internal_channel_size,
        })
    }

    /// Main entry point for the scheduler.
    pub async fn work(mut self) -> Result<(), SchedulerError> {
        // First, check the recovery state.
        let recovery_state = self.check_recovery_state().await.map_err(|e| {
            log::error!("Failed to check the state from last run.");
            if let Err(e) = self.rec_tx.send(RecoveryState::Error) {
                return SchedulerError::RecoverySendFailed(e.0);
            };
            e
        })?;
        self.rec_tx.send(recovery_state)?;
        match recovery_state {
            RecoveryState::Fresh => log::info!("Type `run` to begin running jobs."),
            RecoveryState::Finished => log::info!(
                "Found a finished run. \n\
                Type `run` to begin running jobs and overwrite the existing data, \
                or `exit` to cancel."
            ),
            RecoveryState::GracefullyStopped => log::info!(
                "Found a gracefully stopped run. \n\
                Type `run` to resume running jobs from the last run, or `help` for additional options."
            ),
            RecoveryState::AbortedUnchecked => log::info!(
                "Found an aborted run. \n\
                Type `check` to check if the data is recoverable, \
                `run` to start a new run and overwrite the existing data, \
                or `help` for additional options."
            ),
            _ => unreachable!("Unexpected recovery state: {recovery_state:?}"),
        }
        // Then, wait for the UI to decide what to do next.
        loop {
            self.ctrl_rx.changed().await?;
            let ctrl_event = self.ctrl_rx.borrow_and_update().clone();
            match ctrl_event {
                ControlEvent::Check { primary_ub } => {
                    let consistent = self.check_consistency(primary_ub).await.map_err(|e| {
                        log::error!("Failed to check data consistency: {e}");
                        if let Err(e) = self.rec_tx.send(RecoveryState::Error) {
                            return SchedulerError::RecoverySendFailed(e.0);
                        };
                        e
                    })?;
                    let state_after_check = match (recovery_state, consistent) {
                        (RecoveryState::AbortedUnchecked, true) => RecoveryState::AbortedChecked,
                        (RecoveryState::GracefullyStopped, true) => {
                            RecoveryState::GracefullyStoppedChecked
                        }
                        (_, true) => unreachable!(
                            "Ran `check_consistency` in an unexpected state: {recovery_state:?}"
                        ),
                        (_, false) => RecoveryState::MissingData,
                    };
                    self.rec_tx.send(state_after_check)?;
                    match state_after_check {
                        RecoveryState::MissingData => log::info!(
                            "Some data is corrupted or missing. \n\
                            Type `run` to start a new run and overwrite the existing data, or `exit` to cancel."
                        ),
                        RecoveryState::AbortedChecked => log::info!(
                            "The data is recoverable. \n\
                            Type `run` to rebuild and resume running jobs from the last run, \
                            or `help` for additional options."
                        ),
                        RecoveryState::GracefullyStoppedChecked => log::info!(
                            "No inconsistencies were found. \n\
                                Type `run` to resume running jobs from the last run, or `help` for additional options."
                        ),
                        _ => unreachable!(
                            "Unexpected recovery state after consistency check: {consistent:?}"
                        ),
                    }
                }
                ControlEvent::CleanRun { primary_ub } => {
                    return self.run(primary_ub, RunMode::Clean).await;
                }
                ControlEvent::RebuildRun { primary_ub } => {
                    return self.run(primary_ub, RunMode::Rebuild).await;
                }
                ControlEvent::RestoreRun { primary_ub } => {
                    return self.run(primary_ub, RunMode::Restore).await;
                }
                ControlEvent::Abort => {
                    // Decided to not start a new run.
                    return Ok(());
                }
                _ => {
                    // Other control events should not be passed in here.
                    log::warn!("Received an unexpected control event: {ctrl_event:?}");
                    return Err(SchedulerError::UnexpectedControlEvent(ctrl_event));
                }
            }
        }
    }

    /// Find out the recovery state.
    pub async fn check_recovery_state(&self) -> Result<RecoveryState, SchedulerError> {
        let storage = &self.storage;
        let meta_storage = &self.meta_storage;
        let meta_conn = meta_storage.client().await?;

        // Get footprints from both storages.
        let data_footprint = storage.get_footprint().await?;
        let meta_footprint = meta_storage.get_footprint(&meta_conn, "global").await?;
        // Early return if the state can be inferred through the footprints.
        match (&data_footprint, &meta_footprint) {
            (Some(df), Some(mf)) if df == mf && df.starts_with("F@") => {
                // Both storages have the same footprint, and it is a "finished" one.
                return Ok(RecoveryState::Finished);
            }
            (Some(df), Some(mf)) if df == mf && df.starts_with("S@") => {
                // Both storages have the same footprint, and it is a "stopped" one.
                return Ok(RecoveryState::GracefullyStopped);
            }
            _ => (),
        }

        let resolution = meta_storage.get_primary_resolution(&meta_conn).await?;
        if resolution.is_none() {
            // No resolution found, so the metadata storage is empty.
            return Ok(RecoveryState::Fresh);
        }
        // Fall back to `AbortedUnchecked`: the metadata storage has some data,
        // but it is not consistent with the data storage.
        Ok(RecoveryState::AbortedUnchecked)
    }

    /// Run a check on the data consistency between the data storage and the metadata storage.
    /// Return `true` if the data storage holds all needed data to restore,
    /// or `false` if it does not.
    ///
    /// This should be called only when the recovery state is either
    /// `AbortedUnchecked` or `GracefullyStopped`.
    async fn check_consistency(&self, primary_ub: usize) -> Result<bool, SchedulerError> {
        for schedule in &self.schedules {
            if !schedule.check_consistency(primary_ub).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub async fn update_ui_all(&self, conn: MetaClient<'_>) -> Result<(), SchedulerError> {
        let updates = self.meta_storage.get_ui_updates(conn).await?;
        let mut ui_state = self.ui_state.write().await; // Might break
        for update in updates {
            ui_state.update_ui_state(update)?;
        }
        Ok(())
    }

    async fn run(self, primary_ub: usize, run_mode: RunMode) -> Result<(), SchedulerError> {
        // Set up the initial storage setup and initial tickets for the individual schedulers.
        let mut handles = match run_mode {
            RunMode::Clean => self.run_clean(primary_ub).await?,
            RunMode::Rebuild => self.run_rebuild(primary_ub).await?,
            RunMode::Restore => self.run_restore().await?,
        };

        // match run_mode {
        //     RunMode::Clean => {}
        // }

        let mut returned_states = vec![];
        while let Some(res) = handles.join_next().await {
            // Bubble up JoinError and push the resulting state.
            returned_states.push(res?);
        }

        // Here, we have the following possible states:
        // 1. All jobs finished successfully.
        // 2. All jobs are `Finished` or `Stopped`, and the control signal is a `GracefulStop` event.
        // 3-1. All jobs are `Finished` or `Stopped`, and the control signal is an `Abort` event.
        // 3-2. Some jobs returned an `Error`.
        // On `1`, we set the footprint to "F@{now}".
        // On `2`, we set the footprint to "S@{now}".
        // On `3`, we don't set the footprint.
        let footprint = {
            let now = ::chrono::Local::now();
            if returned_states.iter().all(|&s| s == RunningState::Finished) {
                Some(format!("F@{now}"))
            } else if returned_states
                .iter()
                .all(|&s| s == RunningState::Stopped || s == RunningState::Finished)
                && self.ctrl_rx.borrow().clone() == ControlEvent::GracefulStop
            {
                Some(format!("S@{now}"))
            } else {
                None
            }
        };

        if let Some(footprint) = footprint {
            // Write the footprint to the data storage...
            self.storage.put_footprint(&footprint).await?;
            // ...and to the metadata storage.
            let mut client = self.meta_storage.client().await?;
            let tx = client.transaction().await.map_err(MetaStorageError::from)?;
            self.meta_storage
                .put_footprint(&tx, "global", footprint)
                .await?;
            tx.commit().await.map_err(MetaStorageError::from)?;
        }
        log::info!("All jobs closed.");
        Ok(())
    }

    async fn run_clean(&self, primary_ub: usize) -> Result<JoinSet<RunningState>, SchedulerError> {
        // Wipe the data storage clean.
        self.storage.clear().await?;
        let mut client = self.meta_storage.client().await?;
        let tx = client.transaction().await.map_err(MetaStorageError::from)?;
        self.meta_storage.clear_resolution(&tx).await?;
        self.meta_storage
            .put_primary_resolution(&tx, primary_ub)
            .await?;
        self.meta_storage.clear_tickets(&tx).await?;
        self.meta_storage.put_default_tickets(&tx).await?;
        self.meta_storage.clear_footprint(&tx).await?;
        tx.commit().await.map_err(MetaStorageError::from)?;

        let PreparedSchedules {
            schedules_with_rx: schedule_with_rx,
            peer_txs,
        } = self.prepare_channels();
        let handles = JoinSet::from_iter(schedule_with_rx.into_iter().map(
            |ScheduleWithRx { schedule, peer_rx }| {
                schedule.start_clean(
                    self.service.clone(),
                    self.storage.clone(),
                    self.meta_storage.clone(),
                    self.ui_state.clone(),
                    peer_txs.clone(),
                    peer_rx,
                    self.ctrl_rx.clone(),
                )
            },
        ));
        for peer_tx in peer_txs.into_values() {
            peer_tx.send().await?;
        }

        Ok(handles)
    }

    async fn run_rebuild(
        &self,
        primary_ub: usize,
    ) -> Result<JoinSet<RunningState>, SchedulerError> {
        // * We *trust* the following data to be correct:
        //   - The data storage,
        //   - All dimension resolutions,
        //   - All done tickets.
        // * What we need to do:
        //   - Pull the done tickets and potentially their associated resolutions,
        //   - couple them into internal events `JobSuccess(job, resolution)`,
        //   - wipe the metadata storage once that's done,
        //   - roll the internal events out manually,
        //   - and finally feed the resulting queued tickets to the individual schedulers.

        // Clear the data storage's footprint. (The metadata storage will be cleared later.)
        let mut client = self.meta_storage.client().await?;
        let tx = client.transaction().await.map_err(MetaStorageError::from)?;
        let rebuilders =
            try_join_all(self.schedules.iter().map(|job| job.prepare_rebuild(&tx))).await?;

        self.meta_storage.clear_resolution(&tx).await?;
        self.meta_storage.clear_tickets(&tx).await?;
        self.meta_storage.clear_footprint(&tx).await?;

        self.meta_storage.put_default_tickets(&tx).await?;
        self.meta_storage
            .put_primary_resolution(&tx, primary_ub)
            .await?;
        for rebuilder in &rebuilders {
            rebuilder.explode(primary_ub).await?;
        }
        self.update_ui_all(MetaClient::Transaction(&tx)).await?;

        for rebuilder in rebuilders {
            rebuilder.rebuild().await?;
            self.update_ui_all(MetaClient::Transaction(&tx)).await?;
        }
        tx.commit().await.map_err(MetaStorageError::from)?;
        log::info!("Rebuild complete, starting the run.");

        let PreparedSchedules {
            schedules_with_rx,
            peer_txs,
        } = self.prepare_channels();
        Ok(JoinSet::from_iter(schedules_with_rx.into_iter().map(
            |ScheduleWithRx { schedule, peer_rx }| {
                schedule.start_rebuild(
                    self.service.clone(),
                    self.storage.clone(),
                    self.meta_storage.clone(),
                    self.ui_state.clone(),
                    peer_txs.clone(),
                    peer_rx,
                    self.ctrl_rx.clone(),
                )
            },
        )))
    }

    async fn run_restore(&self) -> Result<JoinSet<RunningState>, SchedulerError> {
        // * The persistent storage is fully trusted.
        // * Just pull the queued tickets, and have the individual schedulers'
        //   initial `ready_to_run` set to them.
        // * We need to clear the footprint only.
        let storage = &self.storage;
        let meta_storage = &self.meta_storage;

        // Clear the data storage's footprint.
        storage.clear_footprint().await?;
        // Clear the metadata storage's footprint.
        {
            let mut client = meta_storage.client().await?;
            let tx = client.transaction().await.map_err(MetaStorageError::from)?;
            meta_storage.clear_footprint(&tx).await?;
            tx.commit().await.map_err(MetaStorageError::from)?;
        }

        let PreparedSchedules {
            schedules_with_rx,
            peer_txs,
        } = self.prepare_channels();
        Ok(JoinSet::from_iter(schedules_with_rx.into_iter().map(
            |ScheduleWithRx { schedule, peer_rx }| {
                schedule.start_restore(
                    self.service.clone(),
                    self.storage.clone(),
                    self.meta_storage.clone(),
                    self.ui_state.clone(),
                    peer_txs.clone(),
                    peer_rx,
                    self.ctrl_rx.clone(),
                )
            },
        )))
    }

    fn prepare_channels(&self) -> PreparedSchedules<'_, Svc, Sto, MSto> {
        let len = self.schedules.len();

        let mut schedules_with_rx = Vec::with_capacity(len);
        let mut peer_txs = HashMap::with_capacity(len);

        self.schedules.iter().for_each(|schedule| {
            let (peer_tx, peer_rx) =
                tokio::sync::mpsc::channel::<PeerEvent>(self.internal_channel_size);
            peer_txs.insert(schedule.id(), PeerEventSender::Up(peer_tx));
            schedules_with_rx.push(ScheduleWithRx::new(schedule.as_ref(), peer_rx));
        });

        PreparedSchedules::new(schedules_with_rx, peer_txs)
    }
}
