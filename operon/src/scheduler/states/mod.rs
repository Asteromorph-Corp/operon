use async_trait::async_trait;

use crate::scheduler::{ControlEvent, SchedulerError};

mod fresh;
mod init;
mod running;
mod stale;

pub enum NextState {
    Next(Box<dyn SchedulerState>),
    Exit,
}

#[async_trait]
pub trait SchedulerState: Send + Sync {
    async fn handle_progress(self: Box<Self>) -> Result<NextState, SchedulerError>;

    async fn handle_control_event(
        self: Box<Self>,
        evt: ControlEvent,
    ) -> Result<NextState, SchedulerError>;
}

impl<T: SchedulerState + Sized + 'static> From<T> for NextState {
    fn from(value: T) -> Self {
        NextState::Next(Box::new(value))
    }
}
