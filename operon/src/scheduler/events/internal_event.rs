use crate::scheduler::SchedulerError;

/// `IndividualScheduler`-worker communication events.
///
/// These are used for communication between individual schedulers and their
/// child worker coroutines.
#[derive(Debug)]
pub enum InternalEvent<J, R> {
    /// A job successfully finished.
    JobSuccess(J, R),
    /// A job failed.
    JobFailure(J, SchedulerError),
    /// All outstanding pool permits were reclaimed and forgotten in the
    /// background, following a `Pause` or graceful `Quit`.
    PermitsReclaimed,
}
