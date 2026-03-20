use std::collections::VecDeque;

use async_trait::async_trait;
use tokio::task::JoinHandle;

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

#[async_trait]
pub trait SchedulerTransition: Send + Sync + 'static {
    fn warn_msg(&self) -> Option<&'static str>;
    async fn execute(self) -> Result<NextState, SchedulerError>;
}

pub struct TransitionState {
    warn_msg: Option<&'static str>,
    handle: JoinHandle<Result<NextState, SchedulerError>>,
    events: VecDeque<ControlEvent>,
}

impl TransitionState {
    pub fn new<T: SchedulerTransition>(transition: T) -> Self {
        let warn_msg = transition.warn_msg();
        let handle = tokio::task::spawn(async move { transition.execute().await });
        Self {
            warn_msg,
            handle,
            events: VecDeque::new(),
        }
    }
}

#[async_trait]
impl SchedulerState for TransitionState {
    async fn handle_progress(mut self: Box<Self>) -> Result<NextState, SchedulerError> {
        if self.handle.is_finished() {
            let mut next = self.handle.await.map_err(SchedulerError::from).flatten()?;
            while let Some(evt) = self.events.pop_front() {
                match next {
                    NextState::Next(state) => {
                        next = state.handle_control_event(evt).await?;
                    }
                    NextState::Exit => return Ok(NextState::Exit),
                }
            }
            return Ok(next);
        }
        Ok(NextState::Next(self))
    }

    async fn handle_control_event(
        mut self: Box<Self>,
        evt: ControlEvent,
    ) -> Result<NextState, SchedulerError> {
        if let Some(msg) = self.warn_msg {
            log::warn!("{msg}")
        }
        self.events.push_back(evt);

        Ok(NextState::Next(self))
    }
}

impl<T: SchedulerState + Sized + 'static> From<T> for NextState {
    fn from(value: T) -> Self {
        NextState::Next(Box::new(value))
    }
}
