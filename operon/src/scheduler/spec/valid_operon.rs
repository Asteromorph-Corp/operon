use crate::scheduler::SchedulerHandler;
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub trait ValidOperon<Svc: OperonService, Sto: OperonStorage> {
    fn scheduler_handler() -> SchedulerHandler<Svc, Sto>;
}
