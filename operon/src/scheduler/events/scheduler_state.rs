pub type SchedulerStateSender = ::tokio::sync::oneshot::Sender<Result<bool, String>>;
pub type SchedulerStateReceiver = ::tokio::sync::oneshot::Receiver<Result<bool, String>>;
