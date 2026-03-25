use std::sync::Arc;

use crate::meta_storage::MetaStorage;
use crate::scheduler::SchedulerHandler;
use crate::schema::SharedProgressMap;
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub struct SchedulerContext<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub service: Arc<Svc>,
    pub storage: Arc<Sto>,
    pub meta_storage: MetaStorage,
    pub handler: SchedulerHandler<Svc, Sto>,
    pub progresses: SharedProgressMap,
}
