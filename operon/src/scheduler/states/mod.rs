use std::collections::VecDeque;
use std::error::Error;

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

pub(super) enum NextState<E> {
    Next(Box<dyn SchedulerState<Error = E>>),
    Exit { exit_ui: bool },
}

#[async_trait]
pub(super) trait SchedulerState: Send + Sync {
    type Error: Error + Send + Sync + 'static;

    async fn handle_progress(self: Box<Self>) -> Result<NextState<Self::Error>, Self::Error>;

    async fn handle_control_event(
        self: Box<Self>,
        evt: ControlEvent,
    ) -> Result<NextState<Self::Error>, Self::Error>;
}

#[async_trait]
pub(super) trait SchedulerTransition: Send + Sync + 'static {
    type Error: Error + Send + Sync + 'static;
    fn warn_msg(&self) -> Option<&'static str>;
    async fn execute(self) -> Result<NextState<Self::Error>, Self::Error>;
}

pub(super) struct TransitionState<E> {
    warn_msg: Option<&'static str>,
    handle: JoinHandle<Result<NextState<E>, E>>,
    events: VecDeque<ControlEvent>,
}

impl<E: Error + Send + Sync + 'static> TransitionState<E> {
    pub fn new<T: SchedulerTransition<Error = E>>(transition: T) -> Self {
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
impl<UErr, SErr, MErr> SchedulerState for TransitionState<SchedulerError<UErr, SErr, MErr>>
where
    UErr: std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
    SErr: Error + Send + Sync + 'static,
    MErr: Error + Send + Sync + 'static,
{
    type Error = SchedulerError<UErr, SErr, MErr>;

    async fn handle_progress(mut self: Box<Self>) -> Result<NextState<Self::Error>, Self::Error> {
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
    ) -> Result<NextState<Self::Error>, Self::Error> {
        if let Some(msg) = self.warn_msg {
            tracing::warn!("{msg}")
        }
        self.events.push_back(evt);

        Ok(NextState::Next(self))
    }
}

impl<UErr, SErr, MErr> NextState<SchedulerError<UErr, SErr, MErr>>
where
    UErr: std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
    SErr: Error + Send + Sync + 'static,
    MErr: Error + Send + Sync + 'static,
{
    /// Boxes a concrete state into the next state.
    pub(super) fn next(
        state: impl SchedulerState<Error = SchedulerError<UErr, SErr, MErr>> + 'static,
    ) -> Self {
        NextState::Next(Box::new(state))
    }
}
