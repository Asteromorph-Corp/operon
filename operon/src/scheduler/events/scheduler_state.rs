pub(crate) type SchedulerStateSender = ::tokio::sync::oneshot::Sender<Result<bool, String>>;
pub(crate) type SchedulerStateReceiver = ::tokio::sync::oneshot::Receiver<Result<bool, String>>;
