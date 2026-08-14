use crate::meta_storage::MetaBackend;
use crate::scheduler::SchedulerHandler;
use crate::service::OperonService;
use crate::storage::OperonStorage;

/// A service and a storage that were generated from the same pipeline.
///
/// `#[derive(OperonService)]` implements this for the pair, which is how
/// [`Operon`](crate::Operon) rejects a service and a storage belonging to different pipelines at
/// compile time.
pub trait ValidOperon<Svc: OperonService, Sto: OperonStorage> {
    /// Builds the handler holding one [`TaskHandler`](crate::scheduler::TaskHandler) per task of
    /// the pipeline.
    fn scheduler_handler<MSto: MetaBackend>() -> SchedulerHandler<Svc, Sto, MSto>;
}
