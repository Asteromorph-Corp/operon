use std::collections::VecDeque;
use std::error::Error;

use async_trait::async_trait;
use tokio_util::task::AbortOnDropHandle;

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

    /// Resolves once the state has progressed on its own, without a control event.
    ///
    /// States that only ever move on a control event should never resolve.
    /// The scheduler loop races this against incoming control events, so implementations must be
    /// cancel-safe.
    async fn wait_progress(&mut self) -> Result<(), Self::Error> {
        std::future::pending().await
    }

    /// Advances the state after [`SchedulerState::wait_progress`] has resolved.
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
    handle: AbortOnDropHandle<Result<NextState<E>, E>>,
    next: Option<NextState<E>>,
    events: VecDeque<ControlEvent>,
}

impl<E: Error + Send + Sync + 'static> TransitionState<E> {
    pub(super) fn new<T: SchedulerTransition<Error = E>>(transition: T) -> Self {
        let warn_msg = transition.warn_msg();
        let handle =
            AbortOnDropHandle::new(tokio::task::spawn(
                async move { transition.execute().await },
            ));
        Self {
            warn_msg,
            handle,
            next: None,
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

    async fn wait_progress(&mut self) -> Result<(), Self::Error> {
        if self.next.is_none() {
            let next = (&mut self.handle)
                .await
                .map_err(SchedulerError::from)
                .flatten()?;
            self.next = Some(next);
        }
        Ok(())
    }

    async fn handle_progress(mut self: Box<Self>) -> Result<NextState<Self::Error>, Self::Error> {
        let Some(mut next) = self.next.take() else {
            return Ok(NextState::Next(self));
        };

        while let Some(evt) = self.events.pop_front() {
            match next {
                NextState::Next(state) => {
                    next = state.handle_control_event(evt).await?;
                }
                NextState::Exit { exit_ui } => return Ok(NextState::Exit { exit_ui }),
            }
        }

        Ok(next)
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
