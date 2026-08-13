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

/// Asymmetric `try_join` helper where `future1` always runs to completion while `future2` may be
/// canceled if `future1` returns an error.
///
/// If `future2` returns an error but `future1` returns its own error afterwards,
/// then the error from `future1` is returned and the error from `future2` is discarded.
async fn asymmetric_try_join<T, U, E>(
    future1: impl Future<Output = Result<T, E>>,
    future2: impl Future<Output = Result<U, E>>,
) -> Result<(T, U), E> {
    let future2 = async move { Ok::<Result<U, E>, E>(future2.await) };
    let (t, u) = try_join(future1, future2).await?;
    let u = u?;
    Ok((t, u))
}

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

        let progresses = SharedProgressMap::from_tasks(&handler.task_ids());

        let scheduler = Scheduler::<Svc, Sto, MSto>::new(
            service,
            storage,
            meta_storage,
            handler,
            progresses.clone(),
            ctrl_rx,
            channel_size,
            ui_mode,
        );
        let ui_loop = UiLoop::new(progresses, log_rx, ctrl_tx, sched_rx, ui_options);

        // Spawn the scheduler thread
        let scheduler_handle =
            { ::tokio::spawn(async move { scheduler.work_and_send(sched_tx).await }) };

        // A UI error kills the scheduler,
        // but the scheduler-handler join error (i.e., a scheduler panic)
        // lets the UI continue to run and report the error.
        //
        // The UI's error will win even if the scheduler errors first.
        asymmetric_try_join::<_, _, OperonError>(
            async {
                ui_loop.run().await?;
                Ok(())
            },
            async {
                scheduler_handle.await?;
                Ok(())
            },
        )
        .await?;

        Ok(())
    }
}
