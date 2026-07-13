use std::sync::Arc;

use futures::future::try_join;

use crate::logger::UiBroadcastLayer;
use crate::meta_storage::{MetaBackend, MetaBackendOptions, PsqlMetaStorage};
use crate::operon::{OperonError, OperonOptions};
use crate::scheduler::{Scheduler, ValidOperon};
use crate::schema::SharedProgressMap;
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::{UiLoop, UiMode, UiOptions};

/// # Operon
///
/// The interface for the Operon library.
///
/// Provided a data storage and a service, calling `run` will start executing the jobs.
pub struct Operon<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    (Svc, Sto): ValidOperon<Svc, Sto>,
{
    service: Arc<Svc>,
    storage: Arc<Sto>,
    options: OperonOptions,
}

impl<Svc, Sto> Operon<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    (Svc, Sto): ValidOperon<Svc, Sto>,
{
    /// Create a new Operon instance with the given storage and service.
    pub fn new(
        service: impl Into<Arc<Svc>>,
        storage: impl Into<Arc<Sto>>,
        options: OperonOptions,
    ) -> Self {
        Self {
            service: service.into(),
            storage: storage.into(),
            options,
        }
    }

    /// Run the Operon instance with the given primary upper bound.
    ///
    /// This function is intended to be called ONCE in the main thread in a binary executable
    /// context. Running this will take over the terminal, so it is strongly discouraged to make
    /// any other writes to `stdout` or `stderr` while this is running.
    /// Instead, you can use the provided macros to log messages to the UI.
    pub async fn run(self) -> Result<(), OperonError> {
        let Operon {
            service,
            storage,
            options,
        } = self;
        let (ui_options, scheduler_options, log_options) = options.split();
        let (channel_size, ui_mode, backend) = scheduler_options.split();

        match backend {
            MetaBackendOptions::Psql(backend) => {
                Self::run_with::<PsqlMetaStorage>(
                    service,
                    storage,
                    backend,
                    channel_size,
                    ui_mode,
                    ui_options,
                    log_options,
                )
                .await
            }
        }
    }

    /// Builds the scheduler and UI over a concrete metadata backend and drives them to completion.
    ///
    /// The backend-generic handler and progress map are constructed here, past the point where
    /// [`run`](Self::run) has resolved `MSto`.
    async fn run_with<MSto: MetaBackend>(
        service: Arc<Svc>,
        storage: Arc<Sto>,
        backend: MSto::Options,
        channel_size: usize,
        ui_mode: UiMode,
        ui_options: UiOptions,
        log_options: crate::logger::LoggerOptions,
    ) -> Result<(), OperonError> {
        // Initialize the logger
        let (log_tx, log_rx) = ::tokio::sync::broadcast::channel(log_options.buffer_size);

        // Initialize the control event and recovery state channel
        let (ctrl_tx, ctrl_rx) = ::tokio::sync::mpsc::channel(64);

        // Initialize the scheduler state channel
        let (sched_tx, sched_rx) = ::tokio::sync::oneshot::channel();

        // Set up the tracing subscriber
        UiBroadcastLayer::new(log_tx, log_options).setup()?;

        let handler = <(Svc, Sto) as ValidOperon<Svc, Sto>>::scheduler_handler::<MSto>();

        let progresses = SharedProgressMap::from_jobs(&handler.job_ids());

        // Create the scheduler
        let scheduler = Scheduler::<Svc, Sto, MSto>::new(
            service,
            storage,
            handler,
            progresses.clone(),
            ctrl_rx,
            sched_tx,
            channel_size,
            ui_mode,
            backend,
        )?;
        let ui_loop = UiLoop::new(progresses, log_rx, ctrl_tx, sched_rx, ui_options);

        // Spawn the scheduler thread
        let scheduler_handle = { ::tokio::spawn(async move { scheduler.work().await }) };

        try_join(
            async {
                ui_loop.run().await?;
                Ok::<_, OperonError>(())
            },
            async {
                // A panic (`JoinError`) is fatal (kills the UI);
                // a scheduler error is reported to the UI and the UI keeps running.
                if let Err(err) = scheduler_handle.await? {
                    tracing::error!("Scheduler exited abnormally: {err}");
                }
                Ok(())
            },
        )
        .await?;

        Ok(())
    }
}
