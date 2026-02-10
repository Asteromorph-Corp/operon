use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::meta_storage::MetaClient;
use crate::scheduler::SchedulerError;
use crate::ui::UiState;

#[async_trait]
pub trait JobRebuilder: Send + Sync + 'static {
    async fn rebuild(
        &self,
        client: MetaClient<'_>,
        ui_state: &RwLock<UiState>,
    ) -> Result<(), SchedulerError>; // `Scheduler::run` 6179~
}
