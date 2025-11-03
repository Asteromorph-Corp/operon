use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tokio::time::Instant;

use crate::meta_storage::{MetaClient, MetaStorage, clear_footprint, get_footprint, put_footprint};
use crate::operon::RunningState;
use crate::scheduler::{
    ControlEvent, ControlEventReceiver, RecoveryState, RecoveryStateSender, RunMode,
    SchedulerError, SchedulerHandler, SchedulerOptions,
};
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::UiState;

/// # Scheduler
///
/// The orchestrating scheduler that manages the individual schedulers.
///
/// It is responsible for:
///
/// * Initialization of the metadata storage,
/// * initialization of the individual schedulers, and
/// * communication between the UI and the individual schedulers.
pub struct Scheduler<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    service: Arc<Svc>,
    storage: Arc<Sto>,
    meta_storage: MetaStorage,
    handler: SchedulerHandler<Svc, Sto>,
    ui_state: Arc<RwLock<UiState>>,
    ctrl_rx: ControlEventReceiver,
    rec_tx: RecoveryStateSender,
    internal_channel_size: usize,
}

impl<Svc, Sto> Scheduler<Svc, Sto>
where
    Sto: OperonStorage,
    Svc: OperonService,
{
    /// Initialize a new scheduler and its associated storages.
    pub fn new(
        service: Arc<Svc>,
        storage: Arc<Sto>,
        handler: SchedulerHandler<Svc, Sto>,
        ui_state: Arc<RwLock<UiState>>,
        ctrl_rx: ControlEventReceiver,
        rec_tx: RecoveryStateSender,
        options: SchedulerOptions,
    ) -> Result<Self, SchedulerError> {
        let (internal_channel_size, meta_storage_options) = options.split();
        let meta_storage = MetaStorage::new(meta_storage_options)?;

        Ok(Self {
            service,
            storage,
            meta_storage,
            handler,
            ui_state,
            ctrl_rx,
            rec_tx,
            internal_channel_size,
        })
    }

    /// Main entry point for the scheduler.
    pub async fn work(mut self) -> Result<(), SchedulerError> {
        self.storage.init().await?;
        self.init_meta_storage().await?;

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
                ControlEvent::Check => {
                    let consistent = self
                        .handler
                        .check_consistency(
                            &self.storage,
                            self.meta_storage.conn().await?.as_client(),
                        )
                        .await
                        .map_err(|e| {
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
                ControlEvent::CleanRun => return self.run(RunMode::Clean).await,
                ControlEvent::RebuildRun => return self.run(RunMode::Rebuild).await,
                ControlEvent::RestoreRun => return self.run(RunMode::Restore).await,
                // Decided to not start a new run.
                ControlEvent::Abort => return Ok(()),
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
        let meta_conn = self.meta_storage.conn().await?;

        // Get footprints from both storages.
        let data_footprint = self.storage.get_footprint().await?;
        let meta_footprint = get_footprint(meta_conn.as_client(), "global").await?;
        // Early return if the state can be inferred through the footprints.
        match (&data_footprint, &meta_footprint) {
            (Some(df), Some(mf)) if df == mf => {
                if df.starts_with("F@") {
                    // Both storages have the same footprint, and it is a "finished" one.
                    return Ok(RecoveryState::Finished);
                } else if df.starts_with("S@") {
                    // Both storages have the same footprint, and it is a "stopped" one.
                    return Ok(RecoveryState::GracefullyStopped);
                } else {
                    // Both storages have the same footprint, but it is neither "finished" nor
                    // "stopped".
                    log::warn!("Unexpected footprint found: {df}, treating the run as aborted.");
                }
            }
            (Some(df), Some(mf)) if df != mf => {
                log::warn!(
                    "Inconsistent footprints between data and metadata storage: \
                    data footprint: {df}, metadata footprint: {mf}. \
                    treating the run as aborted."
                );
            }
            (None, Some(_)) => {
                log::info!(
                    "Footprint found in metadata storage, but not in data storage, treating the run as fresh."
                );
                return Ok(RecoveryState::Fresh);
            }
            _ => (),
        }

        // Fall back to `AbortedUnchecked`: the metadata storage has some data,
        // but it is not consistent with the data storage.
        Ok(RecoveryState::AbortedUnchecked)
    }

    async fn run(self, run_mode: RunMode) -> Result<(), SchedulerError> {
        let start = Instant::now();

        // Set up the initial storage setup and initial tickets for the individual schedulers.
        let mut handles = match run_mode {
            RunMode::Clean => self.run_clean().await?,
            RunMode::Rebuild => self.run_rebuild().await?,
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
        // 2. All jobs are `Finished` or `Stopped`, and the control signal is a `GracefulStop`
        //    event.
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
            let mut conn = self.meta_storage.conn().await?;
            let tx = conn.transaction().await?;
            put_footprint(tx.as_client(), "global", footprint).await?;
            tx.commit().await?;
        }
        log::info!("All jobs closed in: {:?}.", start.elapsed());
        Ok(())
    }

    async fn run_clean(&self) -> Result<JoinSet<RunningState>, SchedulerError> {
        // Wipe the data storage clean.
        self.storage.clear().await?;
        let mut conn = self.meta_storage.conn().await?;
        let tx = conn.transaction().await?;
        self.handler.clear_resolution(tx.as_client()).await?;
        self.handler.clear_tickets(tx.as_client()).await?;
        self.handler.put_default_tickets(tx.as_client()).await?;
        clear_footprint(tx.as_client()).await?;
        tx.commit().await?;

        let handles = self
            .handler
            .prepare_channels(self.internal_channel_size)
            .run_schedulers(
                &self.service,
                &self.storage,
                &self.meta_storage,
                &self.ui_state,
                &self.ctrl_rx,
                true,
            );

        Ok(handles)
    }

    async fn run_rebuild(&self) -> Result<JoinSet<RunningState>, SchedulerError> {
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
        let mut conn = self.meta_storage.conn().await?;
        let tx = conn.transaction().await?;
        let rebuilders = self
            .handler
            .prepare_rebuilders(&self.storage, tx.as_client())
            .await?;

        self.handler.clear_resolution(tx.as_client()).await?;
        self.handler.clear_tickets(tx.as_client()).await?;
        clear_footprint(tx.as_client()).await?;

        self.handler.put_default_tickets(tx.as_client()).await?;
        self.update_ui(tx.as_client()).await?;

        for rebuilder in rebuilders {
            rebuilder
                .rebuild(tx.as_client(), self.ui_state.as_ref())
                .await?;
            self.update_ui(tx.as_client()).await?;
        }
        tx.commit().await?;
        log::info!("Rebuild complete, starting the run.");

        let handles = self
            .handler
            .prepare_channels(self.internal_channel_size)
            .run_schedulers(
                &self.service,
                &self.storage,
                &self.meta_storage,
                &self.ui_state,
                &self.ctrl_rx,
                false,
            );
        Ok(handles)
    }

    async fn run_restore(&self) -> Result<JoinSet<RunningState>, SchedulerError> {
        // * The persistent storage is fully trusted.
        // * Just pull the queued tickets, and have the individual schedulers' initial
        //   `ready_to_run` set to them.
        // * We need to clear the footprint only.
        let storage = &self.storage;
        let meta_storage = &self.meta_storage;

        // Clear the data storage's footprint.
        storage.clear_footprint().await?;
        // Clear the metadata storage's footprint.
        {
            let mut conn = meta_storage.conn().await?;
            let tx = conn.transaction().await?;
            clear_footprint(tx.as_client()).await?;
            tx.commit().await?;
        }

        let handles = self
            .handler
            .prepare_channels(self.internal_channel_size)
            .run_schedulers(
                &self.service,
                &self.storage,
                &self.meta_storage,
                &self.ui_state,
                &self.ctrl_rx,
                false,
            );
        Ok(handles)
    }

    /// An helper function to call `self.spec.init_meta_storage` with a transaction.
    async fn init_meta_storage(&self) -> Result<(), SchedulerError> {
        let mut conn = self.meta_storage.conn().await?;
        let tx = conn.transaction().await?;
        self.handler.init_meta_storage(tx.as_client()).await?;
        tx.commit().await?;
        Ok(())
    }

    /// An helper function to call `self.spec.update_ui` with a `RwLock` write guard.
    async fn update_ui(&self, client: MetaClient<'_>) -> Result<(), SchedulerError> {
        let mut ui_state = self.ui_state.write().await;
        self.handler.update_ui(client, &mut ui_state).await
    }
}
