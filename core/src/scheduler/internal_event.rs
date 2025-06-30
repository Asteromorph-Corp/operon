use crate::{
    misc::{Job, Resolution},
    scheduler::SchedulerError,
};

pub type IntEventSender<J, R> = tokio::sync::mpsc::UnboundedSender<InternalEvent<J, R>>;
pub type IntEventReceiver<J, R> = tokio::sync::mpsc::UnboundedReceiver<InternalEvent<J, R>>;

/// `IndividualScheduler`-worker communication events.
///
/// These are used for communication between individual schedulers and their
/// child worker coroutines.
#[derive(Debug)]
pub enum InternalEvent<J, R>
where
    J: Job,
    R: Resolution,
{
    /// A job successfully finished.
    JobSuccess(J, R),
    /// A job failed.
    JobFailure(J, SchedulerError),
}
