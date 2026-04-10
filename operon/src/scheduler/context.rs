use std::sync::Arc;

use crate::meta_storage::MetaStorage;
use crate::scheduler::SchedulerHandler;
use crate::schema::SharedProgressMap;
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub(super) struct SchedulerContext<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub(super) service: Arc<Svc>,
    pub(super) storage: Arc<Sto>,
    pub(super) meta_storage: MetaStorage,
    pub(super) handler: SchedulerHandler<Svc, Sto>,
    pub(super) progresses: SharedProgressMap,
}
