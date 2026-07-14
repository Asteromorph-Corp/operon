use std::collections::VecDeque;

use async_trait::async_trait;
use tokio::task::JoinHandle;

use crate::scheduler::SchedulerError;
use crate::scheduler::events::ControlEvent;

mod clean;
mod fresh;
mod init;
mod rebuild;
mod running;
mod stale;
mod start;

pub(super) use init::InitTransition;

pub(super) enum NextState<MErr> {
    Next(Box<dyn SchedulerState<MErr>>),
    Exit { exit_ui: bool },
}

#[async_trait]
pub(super) trait SchedulerState<MErr>: Send + Sync {
    async fn handle_progress(self: Box<Self>) -> Result<NextState<MErr>, SchedulerError<MErr>>;

    async fn handle_control_event(
        self: Box<Self>,
        evt: ControlEvent,
    ) -> Result<NextState<MErr>, SchedulerError<MErr>>;
}

#[async_trait]
pub(super) trait SchedulerTransition<MErr>: Send + Sync + 'static {
    fn warn_msg(&self) -> Option<&'static str>;
    async fn execute(self) -> Result<NextState<MErr>, SchedulerError<MErr>>;
}

pub(super) struct TransitionState<MErr> {
    warn_msg: Option<&'static str>,
    handle: JoinHandle<Result<NextState<MErr>, SchedulerError<MErr>>>,
    events: VecDeque<ControlEvent>,
}

impl<MErr: Send + 'static> TransitionState<MErr> {
    pub fn new<T: SchedulerTransition<MErr>>(transition: T) -> Self {
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
impl<MErr: Send + Sync + 'static> SchedulerState<MErr> for TransitionState<MErr> {
    async fn handle_progress(mut self: Box<Self>) -> Result<NextState<MErr>, SchedulerError<MErr>> {
        if self.handle.is_finished() {
            let mut next = self.handle.await.map_err(SchedulerError::from).flatten()?;
            while let Some(evt) = self.events.pop_front() {
                match next {
                    NextState::Next(state) => {
                        next = state.handle_control_event(evt).await?;
                    }
                    NextState::Exit { exit_ui } => return Ok(NextState::Exit { exit_ui }),
                }
            }
            return Ok(next);
        }
        Ok(NextState::Next(self))
    }

    async fn handle_control_event(
        mut self: Box<Self>,
        evt: ControlEvent,
    ) -> Result<NextState<MErr>, SchedulerError<MErr>> {
        if let Some(msg) = self.warn_msg {
            tracing::warn!("{msg}")
        }
        self.events.push_back(evt);

        Ok(NextState::Next(self))
    }
}

impl<MErr> NextState<MErr> {
    /// Boxes a concrete state into the next state.
    pub(super) fn next(state: impl SchedulerState<MErr> + 'static) -> Self {
        NextState::Next(Box::new(state))
    }
}
