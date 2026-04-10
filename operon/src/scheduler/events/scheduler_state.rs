pub type SchedulerStateSender = ::tokio::sync::oneshot::Sender<bool>;
pub type SchedulerStateReceiver = ::tokio::sync::oneshot::Receiver<bool>;
