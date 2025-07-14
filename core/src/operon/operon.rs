use std::sync::Arc;

use futures::future::try_join;
use tokio::sync::RwLock;

use crate::{
    operon::{OperonError, OperonOptions},
    scheduler::{ControlEvent, RecoveryState, Scheduler, SchedulerHandler},
    service::OperonService,
    storage::OperonStorage,
    ui::{UiLogger, UiLoop, UiState},
};

/// # Operon
///
/// The interface for the Operon library.
///
/// Provided a data storage and a service, calling `run` will start executing the jobs.
pub struct Operon<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    service: Arc<Svc>,
    storage: Arc<Sto>,
    options: OperonOptions,
}

impl<Svc, Sto> Operon<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    /// Create a new Operon instance with the given storage and service.
    pub fn new(
        service: ::std::sync::Arc<Svc>,
        storage: ::std::sync::Arc<Sto>,
        options: OperonOptions,
    ) -> Self {
        Self {
            service,
            storage,
            options,
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
        handler: SchedulerHandler<Svc, Sto>,
        primary_ub: usize,
    ) -> Result<(), OperonError> {
        let (scheduler_options, log_options) = self.options.split();

        // Initialize the logger
        let (log_tx, log_rx) = ::tokio::sync::broadcast::channel(log_options.buffer_size);

        // Initialize the control event and recovery state channel
        let (ctrl_tx, ctrl_rx) = ::tokio::sync::watch::channel(ControlEvent::Start);
        let (rec_tx, rec_rx) = ::tokio::sync::watch::channel(RecoveryState::Unknown);

        // Set up the logger
        UiLogger::new(log_tx, log_options.level, log_options.dump)
            .setup(::log::LevelFilter::Trace)?;
        let ui_state = Arc::new(RwLock::new(UiState::from_jobs(&handler.job_handlers)));

        // Create the scheduler
        let scheduler = Scheduler::<Svc, Sto>::new(
            self.service,
            self.storage,
            handler,
            ui_state.clone(),
            ctrl_rx,
            rec_tx,
            scheduler_options,
        )?;
        let ui_loop = UiLoop::new(ui_state, primary_ub, log_rx, ctrl_tx, rec_rx);

        // Spawn the scheduler thread
        let scheduler_handle = { ::tokio::spawn(async move { scheduler.work().await }) };

        try_join(
            async {
                ui_loop.run().await?;
                Ok::<_, OperonError>(())
            },
            async {
                scheduler_handle.await??;
                Ok(())
            },
        )
        .await?;

        Ok(())
    }
}
