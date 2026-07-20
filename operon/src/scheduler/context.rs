use std::sync::Arc;

use crate::meta_storage::MetaBackend;
use crate::scheduler::SchedulerHandler;
use crate::schema::SharedProgressMap;
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub(super) struct SchedulerContext<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    pub(super) service: Arc<Svc>,
    pub(super) storage: Arc<Sto>,
    pub(super) meta_storage: MSto,
    pub(super) handler: SchedulerHandler<Svc, Sto, MSto>,
    pub(super) progresses: SharedProgressMap,
}
