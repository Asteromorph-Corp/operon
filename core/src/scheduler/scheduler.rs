use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{
    error::OperonError,
    meta_storage::{MetaStorage, MetaStorageOptions},
    promoter::Promoter,
    scheduler::{
        ControlEvent, ControlEventReceiver, RecoveryState, RecoveryStateSender, RunMode,
        SchedulerError,
    },
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
pub struct Scheduler<Sto, Svc> {
    promoter: Box<dyn Promoter<Sto, Svc>>,
    storage: Arc<Sto>,
    service: Arc<Svc>,
    meta_storage: Arc<MetaStorage>,
    ui_state: Arc<RwLock<UiState>>,
    ctrl_rx: ControlEventReceiver,
    rec_tx: RecoveryStateSender,
}

impl<Sto, Svc> Scheduler<Sto, Svc> {
    /// Initialize a new scheduler and its associated storages.
    pub async fn new(
        promoter: impl Promoter<Sto, Svc> + 'static,
        storage: Arc<Sto>,
        service: Arc<Svc>,
        ui_state: Arc<RwLock<UiState>>,
        ctrl_rx: ControlEventReceiver,
        rec_tx: RecoveryStateSender,
        options: MetaStorageOptions,
    ) -> Result<Self, SchedulerError> {
        let promoter = Box::new(promoter);
        let meta_storage = Arc::new(MetaStorage::new(options)?);
        meta_storage.init(promoter.as_ref()).await?;

        Ok(Self {
            promoter,
            storage,
            service,
            meta_storage,
            ui_state,
            ctrl_rx,
            rec_tx,
        })
    }

    // /// Main entry point for the scheduler.
    // pub async fn work(mut self) -> Result<(), SchedulerError> {
    //     // First, check the recovery state.
    //     let recovery_state = self.check_recovery_state().await.map_err(|e| {
    //         log::error!("Failed to check the state from last run.");
    //         if let Err(e) = self.rec_tx.send(RecoveryState::Error)? {
    //             return e;
    //         };
    //         e
    //     })?;
    //     self.rec_tx.send(recovery_state)?;
    //     match recovery_state {
    //         RecoveryState::Fresh => {
    //             log::info!("Type `run` to begin running jobs.");
    //         }
    //         RecoveryState::Finished => {
    //             log::info!(
    //                 "Found a finished run. \n\
    //                 Type `run` to begin running jobs and overwrite the existing data, \
    //                 or `exit` to cancel."
    //             );
    //         }
    //         RecoveryState::GracefullyStopped => {
    //             log::info!(
    //                 "Found a gracefully stopped run. \n\
    //                 Type `run` to resume running jobs from the last run, or `help` for additional options."
    //             )
    //         }
    //         RecoveryState::AbortedUnchecked => {
    //             log::info!(
    //                 "Found an aborted run. \n\
    //                 Type `check` to check if the data is recoverable, \
    //                 `run` to start a new run and overwrite the existing data, \
    //                 or `help` for additional options."
    //             );
    //         }
    //         _ => unreachable!("Unexpected recovery state: {recovery_state:?}"),
    //     }
    //     // Then, wait for the UI to decide what to do next.
    //     loop {
    //         self.ctrl_rx.changed().await.map_err(scheduler_error)?;
    //         let ctrl_event = self.ctrl_rx.borrow_and_update().clone();
    //         match ctrl_event {
    //             ControlEvent::Check { primary_ub } => {
    //                 let consistent = self.check_consistency(primary_ub).await.map_err(|e| {
    //                     log::error!("Failed to check data consistency: {e}");
    //                     if let Err(e) = self.rec_tx.send(RecoveryState::Error)? {
    //                         return e;
    //                     };
    //                     e
    //                 })?;
    //                 let state_after_check = if consistent {
    //                     match recovery_state {
    //                         RecoveryState::AbortedUnchecked => RecoveryState::AbortedChecked,
    //                         RecoveryState::GracefullyStopped => {
    //                             RecoveryState::GracefullyStoppedChecked
    //                         }
    //                         _ => unreachable!(
    //                             "Ran `check_consistency` in an unexpected state: {recovery_state:?}"
    //                         ),
    //                     }
    //                 } else {
    //                     RecoveryState::MissingData
    //                 };
    //                 self.rec_tx.send(state_after_check)?;
    //                 match state_after_check {
    //                     RecoveryState::MissingData => {
    //                         log::info!(
    //                             "Some data is corrupted or missing. \n\
    //                             Type `run` to start a new run and overwrite the existing data, or `exit` to cancel."
    //                         )
    //                     }
    //                     RecoveryState::AbortedChecked => {
    //                         log::info!(
    //                             "The data is recoverable. \n\
    //                             Type `run` to rebuild and resume running jobs from the last run, \
    //                             or `help` for additional options."
    //                         )
    //                     }
    //                     RecoveryState::GracefullyStoppedChecked => {
    //                         log::info!(
    //                             "No inconsistencies were found. \n\
    //                             Type `run` to resume running jobs from the last run, or `help` for additional options."
    //                         );
    //                     }
    //                     _ => unreachable!(
    //                         "Unexpected recovery state after consistency check: {consistent:?}"
    //                     ),
    //                 }
    //             }
    //             ControlEvent::CleanRun { primary_ub } => {
    //                 return self.run(primary_ub, RunMode::Clean).await;
    //             }
    //             ControlEvent::RebuildRun { primary_ub } => {
    //                 return self.run(primary_ub, RunMode::Rebuild).await;
    //             }
    //             ControlEvent::RestoreRun { primary_ub } => {
    //                 return self.run(primary_ub, RunMode::Restore).await;
    //             }
    //             ControlEvent::Abort => {
    //                 // Decided to not start a new run.
    //                 return Ok(());
    //             }
    //             _ => {
    //                 // Other control events should not be passed in here.
    //                 log::warn!("Received an unexpected control event: {ctrl_event:?}");
    //                 return Err(SchedulerError::UnexpectedControlEvent(ctrl_event));
    //             }
    //         }
    //     }
    // }

    // /// Find out the recovery state.
    // pub async fn check_recovery_state(&self) -> Result<RecoveryState, SchedulerError> {
    //     // Get footprints from both storages.
    //     let data_footprint = self
    //         .storage
    //         .get_footprint()
    //         .await
    //         .map_err(OperonError::Storage)?;
    //     let meta_footprint = {
    //         let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //         // Read-only, so no transaction needed.
    //         let conn = MetaStorageConnection {
    //             client: &client,
    //             schema: &self.meta_schema,
    //         };
    //         footprint_psql::get_footprint(conn, "global".to_string()).await?
    //     };
    //     // Early return if the state can be inferred through the footprints.
    //     match (&data_footprint, &meta_footprint) {
    //         (Some(df), Some(mf)) if df == mf && df.starts_with("F@") => {
    //             // Both storages have the same footprint, and it is a "finished" one.
    //             return Ok(RecoveryState::Finished);
    //         }
    //         (Some(df), Some(mf)) if df == mf && df.starts_with("S@") => {
    //             // Both storages have the same footprint, and it is a "stopped" one.
    //             return Ok(RecoveryState::GracefullyStopped);
    //         }
    //         _ => {}
    //     }
    //     // Check for `Fresh`: whether the metadata storage holds the primary resolution.
    //     let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //     let conn = MetaStorageConnection {
    //         client: &client,
    //         schema: &self.meta_schema,
    //     };
    //     let resolution =
    //         facts_psql::get_resolution(conn, &masked_dimension::ResolutionRequest::I).await?;
    //     if resolution.is_none() {
    //         // No resolution found, so the metadata storage is empty.
    //         return Ok(RecoveryState::Fresh);
    //     }
    //     // Fall back to `AbortedUnchecked`: the metadata storage has some data,
    //     // but it is not consistent with the data storage.
    //     Ok(RecoveryState::AbortedUnchecked)
    // }

    // /// Run a check on the data consistency between the data storage and the metadata storage.
    // /// Return `true` if the data storage holds all needed data to restore,
    // /// or `false` if it does not.
    // ///
    // /// This should be called only when the recovery state is either
    // /// `AbortedUnchecked` or `GracefullyStopped`.
    // async fn check_consistency(&self, primary_ub: usize) -> Result<bool> {
    //     '_a: {
    //         // Pull the primary resolution from the metadata storage...
    //         let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //         let conn = MetaStorageConnection {
    //             client: &client,
    //             schema: &self.meta_schema,
    //         };
    //         let Some(i_ub) =
    //             facts_psql::get_resolution(conn, &masked_dimension::ResolutionRequest::I)
    //                 .await?
    //                 .and_then(|r| {
    //                     if let masked_dimension::Resolution::I(i) = r {
    //                         Some(i)
    //                     } else {
    //                         None
    //                     }
    //                 })
    //         else {
    //             // This is technically unreachable, because we check this same value
    //             // in `check_recovery_state`.
    //             info!("No primary resolution found in the metadata storage.");
    //             return Ok(false);
    //         };
    //         // ...and check if the data storage holds all the data for it.
    //         for i in 0..i_ub.max(primary_ub) {
    //             if self
    //                 .storage
    //                 .get_a(i)
    //                 .await
    //                 .map_err(operon::error::OperonError::Storage)?
    //                 .is_none()
    //             {
    //                 info!("Data storage does not hold `A_{i}`.");
    //                 return Ok(false);
    //             }
    //         }
    //         // Additionally check if the primary resolution agrees with the given upper bound.
    //         if i_ub != primary_ub {
    //             warn!(
    //                 "Previous run's upper bound `{i_ub}` is different from the current run's upper bound `{primary_ub}`. \n\
    //                         If you overwrote the primary data, consider running `run --fresh` to overwrite the existing data, \
    //                         otherwise the resulting data may be inconsistent. \n\
    //                         If you want to keep the existing data, and intendedly set the upper bound to `{primary_ub}`, \
    //                         you may ignore this warning."
    //             );
    //         }
    //     }
    //     '_b: {
    //         // Pull the "done" beta jobs from the metadata storage...
    //         let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //         let conn = MetaStorageConnection {
    //             client: &client,
    //             schema: &self.meta_schema,
    //         };
    //         let Some(beta_jobs) = tickets_psql::get_all_done::<_, BetaTicket>(conn)
    //             .await?
    //             .iter()
    //             .map(|t| {
    //                 t.resolve().and_then(|job| {
    //                     if let job::Job::Beta { i } = job {
    //                         Some((i,))
    //                     } else {
    //                         None
    //                     }
    //                 })
    //             })
    //             .collect::<Option<Vec<_>>>()
    //         else {
    //             info!("Some `beta` tickets are corrupt in the metadata storage.");
    //             return Ok(false);
    //         };
    //         // ...and map them with the dimensions they spawned...
    //         let mut b_tags = vec![];
    //         for (i,) in beta_jobs {
    //             let Some(j_ub) =
    //                 facts_psql::get_resolution(conn, &masked_dimension::ResolutionRequest::J(i))
    //                     .await?
    //                     .and_then(|r| {
    //                         if let masked_dimension::Resolution::J(j_ub, _) = r {
    //                             Some(j_ub)
    //                         } else {
    //                             None
    //                         }
    //                     })
    //             else {
    //                 info!("No `j` resolution found for `beta_{i}` in the metadata storage.");
    //                 return Ok(false);
    //             };
    //             for j in 0..j_ub {
    //                 b_tags.push((i, j));
    //             }
    //         }
    //         // ...and check if the data storage holds all the data for them.
    //         for (i, j) in b_tags {
    //             if self
    //                 .storage
    //                 .get_b(i, j)
    //                 .await
    //                 .map_err(operon::error::OperonError::Storage)?
    //                 .is_none()
    //             {
    //                 info!("Data storage does not hold `B_{i},{j}`.");
    //                 return Ok(false);
    //             }
    //         }
    //     }
    //     '_c: {
    //         // Pull the "done" gamma jobs from the metadata storage...
    //         let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //         let conn = MetaStorageConnection {
    //             client: &client,
    //             schema: &self.meta_schema,
    //         };
    //         let Some(gamma_jobs) = tickets_psql::get_all_done::<_, GammaTicket>(conn)
    //             .await?
    //             .iter()
    //             .map(|t| {
    //                 t.resolve().and_then(|job| {
    //                     if let job::Job::Gamma { i } = job {
    //                         Some((i,))
    //                     } else {
    //                         None
    //                     }
    //                 })
    //             })
    //             .collect::<Option<Vec<_>>>()
    //         else {
    //             info!("Some `gamma` tickets are corrupt in the metadata storage.");
    //             return Ok(false);
    //         };
    //         // ...and map them with the dimensions they spawned...
    //         let mut c_tags = vec![];
    //         for (i,) in gamma_jobs {
    //             let Some(k_ub) =
    //                 facts_psql::get_resolution(conn, &masked_dimension::ResolutionRequest::K(i))
    //                     .await?
    //                     .and_then(|r| {
    //                         if let masked_dimension::Resolution::K(k_ub, _) = r {
    //                             Some(k_ub)
    //                         } else {
    //                             None
    //                         }
    //                     })
    //             else {
    //                 info!("No `k` resolution found for `gamma_{i}` in the metadata storage.");
    //                 return Ok(false);
    //             };
    //             for k in 0..k_ub {
    //                 c_tags.push((i, k));
    //             }
    //         }
    //         // ...and check if the data storage holds all the data for them.
    //         for (i, k) in c_tags {
    //             if self
    //                 .storage
    //                 .get_c(i, k)
    //                 .await
    //                 .map_err(operon::error::OperonError::Storage)?
    //                 .is_none()
    //             {
    //                 info!("Data storage does not hold `G_{i},{k}`.");
    //                 return Ok(false);
    //             }
    //         }
    //     }
    //     '_d: {
    //         // Pull the "done" delta jobs from the metadata storage...
    //         let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //         let conn = MetaStorageConnection {
    //             client: &client,
    //             schema: &self.meta_schema,
    //         };
    //         let Some(delta_jobs) = tickets_psql::get_all_done::<_, DeltaTicket>(conn)
    //             .await?
    //             .iter()
    //             .map(|t| {
    //                 t.resolve().and_then(|job| {
    //                     if let job::Job::Delta { i, j, k } = job {
    //                         Some((i, j, k))
    //                     } else {
    //                         None
    //                     }
    //                 })
    //             })
    //             .collect::<Option<Vec<_>>>()
    //         else {
    //             info!("Some `delta` tickets are corrupt in the metadata storage.");
    //             return Ok(false);
    //         };
    //         // ...and check if the data storage holds all the data for them.
    //         for (i, j, k) in delta_jobs {
    //             if self
    //                 .storage
    //                 .get_d(i, j, k)
    //                 .await
    //                 .map_err(operon::error::OperonError::Storage)?
    //                 .is_none()
    //             {
    //                 info!("Data storage does not hold `D_{i},{j},{k}`.");
    //                 return Ok(false);
    //             }
    //         }
    //     }
    //     '_e: {
    //         // Pull the "done" epsilon jobs from the metadata storage...
    //         let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //         let conn = MetaStorageConnection {
    //             client: &client,
    //             schema: &self.meta_schema,
    //         };
    //         let Some(epsilon_jobs) = tickets_psql::get_all_done::<_, EpsilonTicket>(conn)
    //             .await?
    //             .iter()
    //             .map(|t| {
    //                 t.resolve().and_then(|job| {
    //                     if let job::Job::Epsilon { i, k } = job {
    //                         Some((i, k))
    //                     } else {
    //                         None
    //                     }
    //                 })
    //             })
    //             .collect::<Option<Vec<_>>>()
    //         else {
    //             info!("Some `epsilon` tickets are corrupt in the metadata storage.");
    //             return Ok(false);
    //         };
    //         // ...and check if the data storage holds all the data for them.
    //         for (i, k) in epsilon_jobs {
    //             if self
    //                 .storage
    //                 .get_e(i, k)
    //                 .await
    //                 .map_err(operon::error::OperonError::Storage)?
    //                 .is_none()
    //             {
    //                 info!("Data storage does not hold `E_{i},{k}`.");
    //                 return Ok(false);
    //             }
    //         }
    //     }
    //     '_f: {
    //         // Pull the "done" zeta jobs from the metadata storage...
    //         let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //         let conn = MetaStorageConnection {
    //             client: &client,
    //             schema: &self.meta_schema,
    //         };
    //         let Some(zeta_jobs) = tickets_psql::get_all_done::<_, ZetaTicket>(conn)
    //             .await?
    //             .iter()
    //             .map(|t| {
    //                 t.resolve().and_then(|job| {
    //                     if let job::Job::Zeta { i } = job {
    //                         Some((i,))
    //                     } else {
    //                         None
    //                     }
    //                 })
    //             })
    //             .collect::<Option<Vec<_>>>()
    //         else {
    //             info!("Some `zeta` tickets are corrupt in the metadata storage.");
    //             return Ok(false);
    //         };
    //         // ...and check if the data storage holds all the data for them.
    //         for (i,) in zeta_jobs {
    //             if self
    //                 .storage
    //                 .get_f(i)
    //                 .await
    //                 .map_err(operon::error::OperonError::Storage)?
    //                 .is_none()
    //             {
    //                 info!("Data storage does not hold `F_{i}`.");
    //                 return Ok(false);
    //             }
    //         }
    //     }
    //     Ok(true)
    // }

    // async fn update_ui_all<Cl>(&self, conn: MetaStorageConnection<'_, Cl>) -> Result<()>
    // where
    //     Cl: MetaClient,
    // {
    //     // Update the UI state with the current status of all individual schedulers.
    //     let beta_counts = tickets_psql::get_beta_status(conn).await?;
    //     let beta_state = if beta_counts.1 + beta_counts.2 == 0 {
    //         RunningState::Finished
    //     } else {
    //         RunningState::Running
    //     };
    //     let gamma_counts = tickets_psql::get_gamma_status(conn).await?;
    //     let gamma_state = if gamma_counts.1 + gamma_counts.2 == 0 {
    //         RunningState::Finished
    //     } else {
    //         RunningState::Running
    //     };
    //     let delta_counts = tickets_psql::get_delta_status(conn).await?;
    //     let delta_state = if delta_counts.1 + delta_counts.2 == 0 {
    //         RunningState::Finished
    //     } else {
    //         RunningState::Running
    //     };
    //     let epsilon_counts = tickets_psql::get_epsilon_status(conn).await?;
    //     let epsilon_state = if epsilon_counts.1 + epsilon_counts.2 == 0 {
    //         RunningState::Finished
    //     } else {
    //         RunningState::Running
    //     };
    //     let zeta_counts = tickets_psql::get_zeta_status(conn).await?;
    //     let zeta_state = if zeta_counts.1 + zeta_counts.2 == 0 {
    //         RunningState::Finished
    //     } else {
    //         RunningState::Running
    //     };

    //     ui::update_ui_state(
    //         &self.ui_state.clone(),
    //         ui::UiStateUpdate::Beta((
    //             beta_counts.0,
    //             beta_counts.1,
    //             beta_counts.2,
    //             beta_state,
    //             false,
    //         )),
    //     )
    //     .await;
    //     ui::update_ui_state(
    //         &self.ui_state.clone(),
    //         ui::UiStateUpdate::Gamma((
    //             gamma_counts.0,
    //             gamma_counts.1,
    //             gamma_counts.2,
    //             gamma_state,
    //             false,
    //         )),
    //     )
    //     .await;
    //     ui::update_ui_state(
    //         &self.ui_state.clone(),
    //         ui::UiStateUpdate::Delta((
    //             delta_counts.0,
    //             delta_counts.1,
    //             delta_counts.2,
    //             delta_state,
    //             false,
    //         )),
    //     )
    //     .await;
    //     ui::update_ui_state(
    //         &self.ui_state.clone(),
    //         ui::UiStateUpdate::Epsilon((
    //             epsilon_counts.0,
    //             epsilon_counts.1,
    //             epsilon_counts.2,
    //             epsilon_state,
    //             false,
    //         )),
    //     )
    //     .await;
    //     ui::update_ui_state(
    //         &self.ui_state.clone(),
    //         ui::UiStateUpdate::Zeta((
    //             zeta_counts.0,
    //             zeta_counts.1,
    //             zeta_counts.2,
    //             zeta_state,
    //             false,
    //         )),
    //     )
    //     .await;
    //     Ok(())
    // }

    // async fn run(mut self, primary_ub: usize, run_mode: RunMode) -> Result<()> {
    //     // Set up the initial storage setup and initial tickets for the individual schedulers.
    //     let initial_data = match run_mode {
    //         RunMode::Clean => {
    //             // Wipe the data storage clean.
    //             self.storage
    //                 .clear()
    //                 .await
    //                 .map_err(operon::error::OperonError::Storage)?;

    //             // Wipe the metadata storage clean.
    //             {
    //                 let mut client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //                 let tx = client.transaction().await.map_err(meta_storage_error)?;
    //                 let conn = MetaStorageConnection {
    //                     client: &tx,
    //                     schema: &self.meta_schema,
    //                 };
    //                 facts_psql::clear(conn).await?;
    //                 facts_psql::put_resolution(conn, &masked_dimension::Resolution::I(primary_ub))
    //                     .await?;
    //                 tickets_psql::clear(conn).await?;
    //                 tickets_psql::put_default_tickets(conn).await?;
    //                 footprint_psql::clear(conn).await?;
    //                 tx.commit().await.map_err(meta_storage_error)?;
    //             }

    //             // No initial tickets.
    //             (vec![], vec![], vec![], vec![], vec![])
    //         }
    //         RunMode::Rebuild => {
    //             // * We *trust* the following data to be correct:
    //             //   - The data storage,
    //             //   - All dimension resolutions,
    //             //   - All done tickets.
    //             // * What we need to do:
    //             //   - Pull the done tickets and potentially their associated resolutions,
    //             //   - couple them into internal events `JobSuccess(job, resolution)`,
    //             //   - wipe the metadata storage once that's done,
    //             //   - roll the internal events out manually,
    //             //   - and finally feed the resulting queued tickets to the individual schedulers.

    //             // Clear the data storage's footprint. (The metadata storage will be cleared later.)
    //             self.storage
    //                 .clear_footprint()
    //                 .await
    //                 .map_err(operon::error::OperonError::Storage)?;
    //             // Pull the done tickets from the metadata storage.
    //             let mut client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //             let tx = client.transaction().await.map_err(meta_storage_error)?;
    //             let conn = MetaStorageConnection {
    //                 client: &tx,
    //                 schema: &self.meta_schema,
    //             };
    //             let beta_events = {
    //                 let mut beta_events = vec![];
    //                 let beta_tickets = tickets_psql::get_all_done::<_, BetaTicket>(conn).await?;
    //                 for ticket in beta_tickets {
    //                     let job::Job::Beta { i } = ticket.resolve().ok_or_else(|| {
    //                         scheduler_error_str("Failed to resolve a beta ticket")
    //                     })?
    //                     else {
    //                         return Err(scheduler_error_str("Expected a beta job"));
    //                     };
    //                     let resolution = facts_psql::get_resolution(
    //                         conn,
    //                         &masked_dimension::ResolutionRequest::J(i),
    //                     )
    //                     .await?
    //                     .ok_or_else(|| {
    //                         meta_storage_error_str(format!("No resolution found for `J_{i}`"))
    //                     })?;
    //                     beta_events.push(InternalEvent::JobSuccess(
    //                         job::Job::Beta { i },
    //                         Some(resolution),
    //                     ));
    //                 }
    //                 beta_events
    //             };
    //             let gamma_events = {
    //                 let mut gamma_events = vec![];
    //                 let gamma_tickets = tickets_psql::get_all_done::<_, GammaTicket>(conn).await?;
    //                 for ticket in gamma_tickets {
    //                     let job::Job::Gamma { i } = ticket.resolve().ok_or_else(|| {
    //                         scheduler_error_str("Failed to resolve a gamma ticket")
    //                     })?
    //                     else {
    //                         return Err(scheduler_error_str("Expected a gamma job"));
    //                     };
    //                     let resolution = facts_psql::get_resolution(
    //                         conn,
    //                         &masked_dimension::ResolutionRequest::K(i),
    //                     )
    //                     .await?
    //                     .ok_or_else(|| {
    //                         meta_storage_error_str(format!("No resolution found for `K_{i}`"))
    //                     })?;
    //                     gamma_events.push(InternalEvent::JobSuccess(
    //                         job::Job::Gamma { i },
    //                         Some(resolution),
    //                     ));
    //                 }
    //                 gamma_events
    //             };
    //             let delta_events = {
    //                 let mut delta_events = vec![];
    //                 let delta_tickets = tickets_psql::get_all_done::<_, DeltaTicket>(conn).await?;
    //                 for ticket in delta_tickets {
    //                     let job::Job::Delta { i, j, k } = ticket.resolve().ok_or_else(|| {
    //                         scheduler_error_str("Failed to resolve a delta ticket")
    //                     })?
    //                     else {
    //                         return Err(scheduler_error_str("Expected a delta job"));
    //                     };
    //                     delta_events
    //                         .push(InternalEvent::JobSuccess(job::Job::Delta { i, j, k }, None));
    //                 }
    //                 delta_events
    //             };
    //             let epsilon_events = {
    //                 let mut epsilon_events = vec![];
    //                 let epsilon_tickets =
    //                     tickets_psql::get_all_done::<_, EpsilonTicket>(conn).await?;
    //                 for ticket in epsilon_tickets {
    //                     let job::Job::Epsilon { i, k } = ticket.resolve().ok_or_else(|| {
    //                         scheduler_error_str("Failed to resolve an epsilon ticket")
    //                     })?
    //                     else {
    //                         return Err(scheduler_error_str("Expected an epsilon job"));
    //                     };
    //                     epsilon_events
    //                         .push(InternalEvent::JobSuccess(job::Job::Epsilon { i, k }, None));
    //                 }
    //                 epsilon_events
    //             };
    //             let zeta_events = {
    //                 let mut zeta_events = vec![];
    //                 let zeta_tickets = tickets_psql::get_all_done::<_, ZetaTicket>(conn).await?;
    //                 for ticket in zeta_tickets {
    //                     let job::Job::Zeta { i } = ticket.resolve().ok_or_else(|| {
    //                         scheduler_error_str("Failed to resolve a zeta ticket")
    //                     })?
    //                     else {
    //                         return Err(scheduler_error_str("Expected a zeta job"));
    //                     };
    //                     zeta_events.push(InternalEvent::JobSuccess(job::Job::Zeta { i }, None));
    //                 }
    //                 zeta_events
    //             };

    //             // Clear the metadata storage.
    //             facts_psql::clear(conn).await?;
    //             tickets_psql::clear(conn).await?;
    //             footprint_psql::clear(conn).await?;

    //             // Roll out the internal events manually.
    //             // Initial setup:
    //             '_a: {
    //                 tickets_psql::put_default_tickets(conn).await?;
    //                 let resolution = masked_dimension::Resolution::I(primary_ub);
    //                 facts_psql::put_resolution(conn, &resolution).await?;
    //                 tickets_psql::explode_beta(conn, &resolution).await?;
    //                 tickets_psql::explode_gamma(conn, &resolution).await?;
    //                 tickets_psql::explode_delta(conn, &resolution).await?;
    //                 tickets_psql::explode_epsilon(conn, &resolution).await?;
    //                 tickets_psql::explode_zeta(conn, &resolution).await?;
    //                 self.update_ui_all(conn).await?;
    //             }
    //             // beta events:
    //             '_b: for event in beta_events {
    //                 let InternalEvent::JobSuccess(job::Job::Beta { i }, Some(resolution)) = event
    //                 else {
    //                     return Err(scheduler_error_str("Expected a beta job success event"));
    //                 };

    //                 // What we would do at a job success:
    //                 facts_psql::put_resolution(conn, &resolution).await?;
    //                 tickets_psql::mark_done_beta(conn, &i).await?;
    //                 // Roll them out to its dependencies:
    //                 tickets_psql::explode_delta(conn, &resolution).await?;
    //                 tickets_psql::raise_dep_delta(
    //                     conn,
    //                     &masked_dimension::I::One(i),
    //                     &masked_dimension::J::All(masked_dimension::I::One(i)),
    //                     &masked_dimension::K::All(masked_dimension::I::One(i)),
    //                 )
    //                 .await?;
    //                 tickets_psql::raise_dep_epsilon(
    //                     conn,
    //                     &masked_dimension::I::One(i),
    //                     &masked_dimension::K::All(masked_dimension::I::One(i)),
    //                 )
    //                 .await?;

    //                 self.update_ui_all(conn).await?;
    //             }
    //             debug!("Rebuilt `beta` jobs.");
    //             '_c: for event in gamma_events {
    //                 let InternalEvent::JobSuccess(job::Job::Gamma { i }, Some(resolution)) = event
    //                 else {
    //                     return Err(scheduler_error_str("Expected a gamma job success event"));
    //                 };

    //                 // What we would do at a job success:
    //                 facts_psql::put_resolution(conn, &resolution).await?;
    //                 tickets_psql::mark_done_gamma(conn, &i).await?;
    //                 // Roll them out to its dependencies:
    //                 tickets_psql::explode_delta(conn, &resolution).await?;
    //                 tickets_psql::explode_epsilon(conn, &resolution).await?;
    //                 tickets_psql::raise_dep_delta(
    //                     conn,
    //                     &masked_dimension::I::One(i),
    //                     &masked_dimension::J::All(masked_dimension::I::One(i)),
    //                     &masked_dimension::K::All(masked_dimension::I::One(i)),
    //                 )
    //                 .await?;
    //                 tickets_psql::raise_dep_zeta(conn, &masked_dimension::I::One(i)).await?;

    //                 self.update_ui_all(conn).await?;
    //             }
    //             debug!("Rebuilt `gamma` jobs.");
    //             '_d: for event in delta_events {
    //                 let InternalEvent::JobSuccess(job::Job::Delta { i, j, k }, None) = event else {
    //                     return Err(scheduler_error_str("Expected a delta job success event"));
    //                 };

    //                 // What we would do at a job success:
    //                 tickets_psql::mark_done_delta(conn, &i, &j, &k).await?;
    //                 // Roll them out to its dependencies:
    //                 tickets_psql::raise_dep_epsilon(
    //                     conn,
    //                     &masked_dimension::I::One(i),
    //                     &masked_dimension::K::One(k),
    //                 )
    //                 .await?;

    //                 self.update_ui_all(conn).await?;
    //             }
    //             debug!("Rebuilt `delta` jobs.");
    //             '_e: for event in epsilon_events {
    //                 let InternalEvent::JobSuccess(job::Job::Epsilon { i, k }, None) = event else {
    //                     return Err(scheduler_error_str("Expected an epsilon job success event"));
    //                 };

    //                 // What we would do at a job success:
    //                 tickets_psql::mark_done_epsilon(conn, &i, &k).await?;
    //                 // Roll them out to its dependencies:
    //                 tickets_psql::raise_dep_zeta(conn, &masked_dimension::I::One(i)).await?;

    //                 self.update_ui_all(conn).await?;
    //             }
    //             debug!("Rebuilt `epsilon` jobs.");
    //             '_f: for event in zeta_events {
    //                 let InternalEvent::JobSuccess(job::Job::Zeta { i }, None) = event else {
    //                     return Err(scheduler_error_str("Expected a zeta job success event"));
    //                 };

    //                 // What we would do at a job success:
    //                 tickets_psql::mark_done_zeta(conn, &i).await?;

    //                 self.update_ui_all(conn).await?;
    //             }
    //             debug!("Rebuilt `zeta` jobs.");
    //             tx.commit().await.map_err(meta_storage_error)?;
    //             info!("Rebuild complete, starting the run.");

    //             // Finally, pull the queued tickets from the metadata storage.
    //             let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //             let conn = MetaStorageConnection {
    //                 client: &client,
    //                 schema: &self.meta_schema,
    //             };
    //             (
    //                 tickets_psql::get_all_queued::<_, BetaTicket>(conn).await?,
    //                 tickets_psql::get_all_queued::<_, GammaTicket>(conn).await?,
    //                 tickets_psql::get_all_queued::<_, DeltaTicket>(conn).await?,
    //                 tickets_psql::get_all_queued::<_, EpsilonTicket>(conn).await?,
    //                 tickets_psql::get_all_queued::<_, ZetaTicket>(conn).await?,
    //             )
    //         }
    //         RunMode::Restore => {
    //             // * The persistent storage is fully trusted.
    //             // * Just pull the queued tickets, and have the individual schedulers'
    //             //   initial `ready_to_run` set to them.
    //             // * We need to clear the footprint only.

    //             // Clear the data storage's footprint.
    //             self.storage
    //                 .clear_footprint()
    //                 .await
    //                 .map_err(operon::error::OperonError::Storage)?;
    //             // Clear the metadata storage's footprint.
    //             {
    //                 let mut client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //                 let tx = client.transaction().await.map_err(meta_storage_error)?;
    //                 let conn = MetaStorageConnection {
    //                     client: &tx,
    //                     schema: &self.meta_schema,
    //                 };
    //                 footprint_psql::clear(conn).await?;
    //                 tx.commit().await.map_err(meta_storage_error)?;
    //             }

    //             // Pull the queued tickets from the metadata storage, and set the initial data.
    //             let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //             let conn = MetaStorageConnection {
    //                 client: &client,
    //                 schema: &self.meta_schema,
    //             };
    //             (
    //                 tickets_psql::get_all_queued::<_, BetaTicket>(conn).await?,
    //                 tickets_psql::get_all_queued::<_, GammaTicket>(conn).await?,
    //                 tickets_psql::get_all_queued::<_, DeltaTicket>(conn).await?,
    //                 tickets_psql::get_all_queued::<_, EpsilonTicket>(conn).await?,
    //                 tickets_psql::get_all_queued::<_, ZetaTicket>(conn).await?,
    //             )
    //         }
    //     };
    //     // Create the mpsc channels.
    //     let (peer_tx_beta, peer_rx_beta) =
    //         ::tokio::sync::mpsc::channel::<PeerEvent>(INTERNAL_CHANNEL_SIZE);
    //     let (peer_tx_gamma, peer_rx_gamma) =
    //         ::tokio::sync::mpsc::channel::<PeerEvent>(INTERNAL_CHANNEL_SIZE);
    //     let (peer_tx_delta, peer_rx_delta) =
    //         ::tokio::sync::mpsc::channel::<PeerEvent>(INTERNAL_CHANNEL_SIZE);
    //     let (peer_tx_epsilon, peer_rx_epsilon) =
    //         ::tokio::sync::mpsc::channel::<PeerEvent>(INTERNAL_CHANNEL_SIZE);
    //     let (peer_tx_zeta, peer_rx_zeta) =
    //         ::tokio::sync::mpsc::channel::<PeerEvent>(INTERNAL_CHANNEL_SIZE);

    //     let _ = self.peer_txs.insert(PeerEventSenders {
    //         to_beta: PeerEventSender::Up(peer_tx_beta),
    //         to_gamma: PeerEventSender::Up(peer_tx_gamma),
    //         to_delta: PeerEventSender::Up(peer_tx_delta),
    //         to_epsilon: PeerEventSender::Up(peer_tx_epsilon),
    //         to_zeta: PeerEventSender::Up(peer_tx_zeta),
    //     });

    //     // Pool sizes are parsed from the config TOML; 1 if not specified.
    //     let meta_pool_cloned = self.meta_pool.clone();
    //     let meta_schema_cloned = self.meta_schema.clone();
    //     let peer_txs_cloned = self.peer_txs.clone().unwrap();
    //     let ctrl_rx_cloned = self.ctrl_rx.clone();
    //     let storage_cloned = self.storage.clone();
    //     let service_cloned = self.service.clone();
    //     let ui_state_cloned = self.ui_state.clone();
    //     self.handles.spawn(async move {
    //         IndividualScheduler::<BetaTicket>::new(
    //             meta_pool_cloned,
    //             meta_schema_cloned,
    //             8,
    //             ui_state_cloned,
    //             peer_txs_cloned,
    //             peer_rx_beta,
    //             ctrl_rx_cloned,
    //         )
    //         .run(storage_cloned, service_cloned, initial_data.0)
    //         .await
    //     });

    //     let meta_pool_cloned = self.meta_pool.clone();
    //     let meta_schema_cloned = self.meta_schema.clone();
    //     let peer_txs_cloned = self.peer_txs.clone().unwrap();
    //     let ctrl_rx_cloned = self.ctrl_rx.clone();
    //     let storage_cloned = self.storage.clone();
    //     let service_cloned = self.service.clone();
    //     let ui_state_cloned = self.ui_state.clone();
    //     self.handles.spawn(async move {
    //         IndividualScheduler::<GammaTicket>::new(
    //             meta_pool_cloned,
    //             meta_schema_cloned,
    //             8,
    //             ui_state_cloned,
    //             peer_txs_cloned,
    //             peer_rx_gamma,
    //             ctrl_rx_cloned,
    //         )
    //         .run(storage_cloned, service_cloned, initial_data.1)
    //         .await
    //     });

    //     let meta_pool_cloned = self.meta_pool.clone();
    //     let meta_schema_cloned = self.meta_schema.clone();
    //     let peer_senders_cloned = self.peer_txs.clone().unwrap();
    //     let ctrl_rx_cloned = self.ctrl_rx.clone();
    //     let storage_cloned = self.storage.clone();
    //     let service_cloned = self.service.clone();
    //     let ui_state_cloned = self.ui_state.clone();
    //     self.handles.spawn(async move {
    //         IndividualScheduler::<DeltaTicket>::new(
    //             meta_pool_cloned,
    //             meta_schema_cloned,
    //             4,
    //             ui_state_cloned,
    //             peer_senders_cloned,
    //             peer_rx_delta,
    //             ctrl_rx_cloned,
    //         )
    //         .run(storage_cloned, service_cloned, initial_data.2)
    //         .await
    //     });

    //     let meta_pool_cloned = self.meta_pool.clone();
    //     let meta_schema_cloned = self.meta_schema.clone();
    //     let peer_senders_cloned = self.peer_txs.clone().unwrap();
    //     let ctrl_rx_cloned = self.ctrl_rx.clone();
    //     let storage_cloned = self.storage.clone();
    //     let service_cloned = self.service.clone();
    //     let ui_state_cloned = self.ui_state.clone();
    //     self.handles.spawn(async move {
    //         IndividualScheduler::<EpsilonTicket>::new(
    //             meta_pool_cloned,
    //             meta_schema_cloned,
    //             4,
    //             ui_state_cloned,
    //             peer_senders_cloned,
    //             peer_rx_epsilon,
    //             ctrl_rx_cloned,
    //         )
    //         .run(storage_cloned, service_cloned, initial_data.3)
    //         .await
    //     });

    //     let meta_pool_cloned = self.meta_pool.clone();
    //     let meta_schema_cloned = self.meta_schema.clone();
    //     let peer_senders_cloned = self.peer_txs.clone().unwrap();
    //     let ctrl_rx_cloned = self.ctrl_rx.clone();
    //     let storage_cloned = self.storage.clone();
    //     let service_cloned = self.service.clone();
    //     let ui_state_cloned = self.ui_state.clone();
    //     self.handles.spawn(async move {
    //         IndividualScheduler::<ZetaTicket>::new(
    //             meta_pool_cloned,
    //             meta_schema_cloned,
    //             1,
    //             ui_state_cloned,
    //             peer_senders_cloned,
    //             peer_rx_zeta,
    //             ctrl_rx_cloned,
    //         )
    //         .run(storage_cloned, service_cloned, initial_data.4)
    //         .await
    //     });

    //     match run_mode {
    //         // If the run is `Clean`, we send the initial resolution to all individual schedulers.
    //         RunMode::Clean => {
    //             let resolution = {
    //                 let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //                 // Read-only, so no transaction needed.
    //                 let conn = MetaStorageConnection {
    //                     client: &client,
    //                     schema: &self.meta_schema,
    //                 };
    //                 facts_psql::get_resolution(conn, &masked_dimension::ResolutionRequest::I)
    //                     .await?
    //                     .ok_or_else(|| {
    //                         meta_storage_error_str("No initial resolution found".to_string())
    //                     })?
    //             };
    //             let masked_dimension::Resolution::I(resolved_i) = resolution else {
    //                 return Err(meta_storage_error_str(
    //                     "Expected initial resolution to be I".to_string(),
    //                 ));
    //             };
    //             // Send the initial resolution, then drop this scheduler's peer_txs.
    //             {
    //                 let peer_txs = self.peer_txs.take().unwrap();
    //                 peer_txs
    //                     .to_beta
    //                     .send(PeerEvent::Resolution(masked_dimension::Resolution::I(
    //                         resolved_i,
    //                     )))
    //                     .await?;
    //                 peer_txs
    //                     .to_gamma
    //                     .send(PeerEvent::Resolution(masked_dimension::Resolution::I(
    //                         resolved_i,
    //                     )))
    //                     .await?;
    //                 peer_txs
    //                     .to_delta
    //                     .send(PeerEvent::Resolution(masked_dimension::Resolution::I(
    //                         resolved_i,
    //                     )))
    //                     .await?;
    //                 peer_txs
    //                     .to_epsilon
    //                     .send(PeerEvent::Resolution(masked_dimension::Resolution::I(
    //                         resolved_i,
    //                     )))
    //                     .await?;
    //                 peer_txs
    //                     .to_zeta
    //                     .send(PeerEvent::Resolution(masked_dimension::Resolution::I(
    //                         resolved_i,
    //                     )))
    //                     .await?;
    //             }
    //         }
    //         // Otherwise, we just drop `peer_txs`.
    //         _ => {
    //             // No initial resolution is sent, as the individual schedulers
    //             // will pull the queued tickets from the metadata storage.
    //             self.peer_txs.take();
    //         }
    //     }

    //     let mut returned_states = vec![];
    //     while let Some(res) = self.handles.join_next().await {
    //         // Bubble up JoinError and push the resulting state.
    //         returned_states.push(res.map_err(scheduler_error)?);
    //     }
    //     // Here, we have the following possible states:
    //     // 1. All jobs finished successfully.
    //     // 2. All jobs are `Finished` or `Stopped`, and the control signal is a `GracefulStop` event.
    //     // 3-1. All jobs are `Finished` or `Stopped`, and the control signal is an `Abort` event.
    //     // 3-2. Some jobs returned an `Error`.
    //     // On `1`, we set the footprint to "F@{now}".
    //     // On `2`, we set the footprint to "S@{now}".
    //     // On `3`, we don't set the footprint.
    //     let footprint = {
    //         let now = ::chrono::Local::now();
    //         if returned_states.iter().all(|&s| s == RunningState::Finished) {
    //             Some(format!("F@{now}"))
    //         } else if returned_states
    //             .iter()
    //             .all(|&s| s == RunningState::Stopped || s == RunningState::Finished)
    //             && self.ctrl_rx.borrow().clone() == ControlEvent::GracefulStop
    //         {
    //             Some(format!("S@{now}"))
    //         } else {
    //             None
    //         }
    //     };
    //     if let Some(footprint) = footprint {
    //         // Write the footprint to the data storage...
    //         self.storage
    //             .put_footprint(&footprint)
    //             .await
    //             .map_err(operon::error::OperonError::Storage)?;
    //         // ...and to the metadata storage.
    //         let mut client = self.meta_pool.get().await.map_err(meta_storage_error)?;
    //         let tx = client.transaction().await.map_err(meta_storage_error)?;
    //         let conn = MetaStorageConnection {
    //             client: &tx,
    //             schema: &self.meta_schema,
    //         };
    //         footprint_psql::put_footprint(conn, "global".to_string(), footprint).await?;
    //         tx.commit().await.map_err(meta_storage_error)?;
    //     }

    //     info!("All jobs closed.");

    //     Ok(())
    // }
}
