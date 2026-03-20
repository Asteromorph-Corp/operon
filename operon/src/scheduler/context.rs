use std::sync::Arc;

use tokio::sync::RwLock;

use crate::meta_storage::MetaStorage;
use crate::scheduler::{ControlEventReceiver, SchedulerHandler};
use crate::service::OperonService;
use crate::storage::OperonStorage;
use crate::ui::UiState;

pub struct SchedulerContext<Svc, Sto>
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    pub service: Arc<Svc>,
    pub storage: Arc<Sto>,
    pub meta_storage: MetaStorage,
    pub handler: SchedulerHandler<Svc, Sto>,
    // TODO: remove these
    pub ui_state: Arc<RwLock<UiState>>,
    pub ctrl_rx: ControlEventReceiver,
}
