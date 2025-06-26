use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{
    meta_storage::MetaStorage,
    operon::{OperonError, OperonOptions},
    scheduler::{ControlEvent, IndividualSchedule, RecoveryState, Scheduler},
    service::OperonService,
    storage::OperonStorage,
    ui::{UiLogger, UiLoop, UiState},
};

/// # Operon
///
/// The interface for the Operon library.
///
/// Provided a data storage and a service, calling `run` will start executing the jobs.
pub struct Operon<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaStorage,
{
    service: Arc<Svc>,
    storage: Arc<Sto>,
    _phantom: std::marker::PhantomData<MSto>,
}

impl<Svc, Sto, MSto> Operon<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaStorage,
{
    /// Create a new Operon instance with the given storage and service.
    pub fn new(service: ::std::sync::Arc<Svc>, storage: ::std::sync::Arc<Sto>) -> Self {
        Self {
            service,
            storage,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Run the Operon instance with the given primary upper bound.
    ///
    /// This function is intended to be called ONCE in the main thread in a binary executable context.
    /// Running this will take over the terminal, so it is strongly discouraged to make any
    /// other writes to `stdout` or `stderr` while this is running.
    /// Instead, you can use the provided macros to log messages to the UI.
    pub async fn run(
        self,
        schedules: Vec<Box<dyn IndividualSchedule<Svc, Sto, MSto>>>,
        primary_ub: usize,
        options: OperonOptions,
    ) -> Result<(), OperonError> {
        let (scheduler_options, log_options) = options.split();

        // Initialize the logger
        let (log_tx, log_rx) = ::tokio::sync::broadcast::channel(log_options.buffer_size);

        // Initialize the control event and recovery state channel
        let (ctrl_tx, ctrl_rx) = ::tokio::sync::watch::channel(ControlEvent::Start);
        let (rec_tx, rec_rx) = ::tokio::sync::watch::channel(RecoveryState::Unknown);

        // Set up the logger
        UiLogger::new(log_tx, log_options.level, log_options.dump)
            .setup(::log::LevelFilter::Trace)?;
        let ui_state = Arc::new(RwLock::new(UiState::default()));

        // Create the scheduler
        let scheduler = Scheduler::<Svc, Sto, MSto>::new(
            self.service,
            self.storage,
            schedules,
            ui_state.clone(),
            ctrl_rx,
            rec_tx,
            scheduler_options,
        )
        .await?;
        let ui_loop = UiLoop::new(ui_state, primary_ub, log_rx, ctrl_tx, rec_rx);

        // Spawn the scheduler thread
        let scheduler_handle = { ::tokio::spawn(async move { scheduler.work().await }) };

        // Main UI loop
        ui_loop.run().await?;

        // By the time the UI exits, the scheduler should have finished
        scheduler_handle.await??;

        Ok(())
    }
}
