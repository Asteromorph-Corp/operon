// use std::sync::Arc;

// use tokio::sync::RwLock;

// use crate::{
//     error::OperonError,
//     scheduler::{ControlEvent, RecoveryState, Scheduler},
//     ui::{LogOptions, UiLogger, UiState},
// };

// /// # Operon
// ///
// /// The interface for the Operon library.
// ///
// /// Provided a data storage and a service, calling `run` will start executing the jobs.
// pub struct Operon<Sto, Svc> {
//     storage: Arc<Sto>,
//     service: Arc<Svc>,
// }

// impl<Sto, Svc> Operon<Sto, Svc> {
//     /// Create a new Operon instance with the given storage and service.
//     pub fn new(storage: ::std::sync::Arc<Sto>, service: ::std::sync::Arc<Svc>) -> Self {
//         Self { storage, service }
//     }

//     /// Run the Operon instance with the given primary upper bound.
//     ///
//     /// This function is intended to be called ONCE in the main thread in a binary executable context.
//     /// Running this will take over the terminal, so it is strongly discouraged to make any
//     /// other writes to `stdout` or `stderr` while this is running.
//     /// Instead, you can use the provided macros to log messages to the UI.
//     pub async fn run(
//         &self,
//         primary_upper_bound: usize,
//         log_options: LogOptions,
//     ) -> Result<(), OperonError> {
//         // Initialize the logger
//         let (log_tx, log_rx) = ::tokio::sync::broadcast::channel(log_options.buffer_size);

//         // Initialize the control event and recovery state channel
//         let (ctrl_tx, ctrl_rx) = ::tokio::sync::watch::channel(ControlEvent::Start);
//         let (rec_tx, rec_rx) = ::tokio::sync::watch::channel(RecoveryState::Unknown);

//         // Set up the logger
//         UiLogger::new(log_tx, log_options.level, log_options.dump)
//             .setup(::log::LevelFilter::Trace)?;
//         let ui_state = Arc::new(RwLock::new(UiState::default()));

//         // Create the scheduler
//         let scheduler = Scheduler::<Sto, Svc>::new(
//             self.storage.clone(),
//             self.service.clone(),
//             ui_state.clone(),
//             ctrl_rx,
//             rec_tx,
//         )
//         .await?;

//         // Spawn the scheduler thread
//         let scheduler_handle = { ::tokio::spawn(async move { scheduler.work().await }) };

//         // Main UI loop
//         ui::run_ui(ui_state, primary_upper_bound, log_rx, ctrl_tx, rec_rx).await?;

//         // By the time the UI exits, the scheduler should have finished
//         scheduler_handle.await.map_err(scheduler_error)??;

//         Ok(())
//     }
// }
