use crate::scheduler::SchedulerError;

/// `IndividualScheduler`-worker communication events.
///
/// These are used for communication between individual schedulers and their
/// child worker coroutines.
#[derive(Debug)]
pub(crate) enum InternalEvent<J, R, UErr, SErr, MErr> {
    /// A job successfully finished.
    JobSuccess(J, R),
    /// A job failed.
    JobFailure(J, SchedulerError<UErr, SErr, MErr>),
}
