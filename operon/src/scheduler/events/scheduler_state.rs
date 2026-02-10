pub type SchedulerStateSender = ::tokio::sync::watch::Sender<bool>;
pub type SchedulerStateReceiver = ::tokio::sync::watch::Receiver<bool>;
pub type SchedulerStateSendError = ::tokio::sync::watch::error::SendError<bool>;
