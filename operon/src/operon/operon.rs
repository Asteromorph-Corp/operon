use std::marker::PhantomData;
use std::sync::Arc;

use futures::future::try_join;

use crate::logger::UiBroadcastLayer;
use crate::meta_storage::{AnyBackend, MetaBackend, MetaBackendOptions};
use crate::operon::{OperonError, OperonOptions};
use crate::scheduler::{Scheduler, ValidOperon};
use crate::schema::SharedProgressMap;
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::UiLoop;

/// # Operon
///
/// The interface for the Operon library.
///
/// Provided a data storage and a service, calling `run` will start executing the jobs.
///
/// `MSto` selects the metadata backend. It defaults to [`AnyBackend`], which is chosen at runtime
/// from the [`OperonOptions`]; pin a concrete backend to select it at compile time instead.
pub struct Operon<Svc, Sto, MSto = AnyBackend>
where
    Svc: OperonService,
    Sto: OperonStorage,
    (Svc, Sto): ValidOperon<Svc, Sto>,
{
    service: Arc<Svc>,
    storage: Arc<Sto>,
    options: OperonOptions,
    _backend: PhantomData<MSto>,
}

impl<Svc, Sto> Operon<Svc, Sto, AnyBackend>
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
            _backend: PhantomData,
        }
    }
}

impl<Svc, Sto, MSto> Operon<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    (Svc, Sto): ValidOperon<Svc, Sto>,
    MSto: MetaBackend<Options = MetaBackendOptions>,
{
    /// Run the Operon instance with the given primary upper bound.
    ///
    /// This function is intended to be called ONCE in the main thread in a binary executable
    /// context. Running this will take over the terminal, so it is strongly discouraged to make
    /// any other writes to `stdout` or `stderr` while this is running.
    /// Instead, you can use the provided macros to log messages to the UI.
    pub async fn run(self) -> Result<(), OperonError<Svc::Error, Sto::Error, MSto::Error>> {
        let Operon {
            service,
            storage,
            options,
            ..
        } = self;
        let (ui_options, scheduler_options, log_options) = options.split();
        let (channel_size, ui_mode, backend) = scheduler_options.split();

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

        // Create the scheduler, resolving the backend from the options.
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
                Ok::<_, OperonError<Svc::Error, Sto::Error, MSto::Error>>(())
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
