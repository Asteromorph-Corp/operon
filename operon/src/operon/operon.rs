use std::sync::Arc;

use futures::future::try_join;

use crate::logger::UiBroadcastLayer;
use crate::meta_storage::MetaBackend;
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
/// Provided a data storage, a service, and a metadata backend, calling `run` will start executing
/// the jobs.
pub struct Operon<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    (Svc, Sto): ValidOperon<Svc, Sto>,
    MSto: MetaBackend,
{
    service: Arc<Svc>,
    storage: Arc<Sto>,
    meta_storage: MSto,
    options: OperonOptions,
}

impl<Svc, Sto, MSto> Operon<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    (Svc, Sto): ValidOperon<Svc, Sto>,
    MSto: MetaBackend,
{
    /// Create a new Operon instance with the given service, storage, and metadata backend.
    pub fn new(
        service: impl Into<Arc<Svc>>,
        storage: impl Into<Arc<Sto>>,
        meta_storage: MSto,
    ) -> Self {
        Self {
            service: service.into(),
            storage: storage.into(),
            meta_storage,
            options: OperonOptions::default(),
        }
    }

    /// Overrides the run settings, replacing any previously set options.
    pub fn with_options(mut self, options: OperonOptions) -> Self {
        self.options = options;
        self
    }

    /// Run the Operon instance.
    pub async fn run(self) -> Result<(), OperonError> {
        let Operon {
            service,
            storage,
            meta_storage,
            options,
        } = self;
        let (ui_options, scheduler_options, log_options) = options.split();
        let (channel_size, ui_mode) = scheduler_options.split();

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

        let scheduler = Scheduler::<Svc, Sto, MSto>::new(
            service,
            storage,
            meta_storage,
            handler,
            progresses.clone(),
            ctrl_rx,
            sched_tx,
            channel_size,
            ui_mode,
        );
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
