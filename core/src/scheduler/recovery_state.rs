use std::fmt;

pub type RecoveryStateReceiver = tokio::sync::watch::Receiver<RecoveryState>;
pub type RecoveryStateSender = tokio::sync::watch::Sender<RecoveryState>;

/// Information about how recoverable the last run was.
/// Recovery methods that can be used to recover lower variants
/// can also be used to recover higher variants.
///
/// Generally, this means that the higher the variant,
/// the more information is available/trustworthy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RecoveryState {
    /// Not yet known.
    Unknown,
    /// The current scheduler made an error.
    Error,
    /// Cannot be recovered at all.
    /// This is the case where the metadata storage is empty (e.g. the scheduler was never run).
    Fresh,
    /// The data storage is corrupt or holds no data.
    /// We cannot recover from this state.
    MissingData,
    /// The last run was successfully finished,
    /// so we cannot resume from that run.
    Finished,
    /// The last run was presumably aborted,
    /// so the storages may be in an inconsistent state.
    /// This happens when:
    /// * the scheduler was stopped abruptly (due to external reasons),
    /// * the scheduler was aborted (i.e. due to an error or a `quit -f` signal),
    /// * the data storage does not support graceful recovery,
    /// * or the data storage is volatile or was changed in between runs.
    ///
    /// Recovery is possible if the "done" data is still available,
    /// so an additional check is needed to determine
    /// whether at least the "done" data is available.
    /// Doing so will resolve this state to `MissingData` or `AbortedChecked`.
    AbortedUnchecked,
    /// The last run was aborted, but both storages agree about the "done" data.
    /// This means that we can rebuild the job queues from
    /// the "done" tickets and all resolutions.
    AbortedChecked,
    /// The last run was gracefully stopped,
    /// so we can trust all the metadata in the storage.
    GracefullyStopped,
    /// The last run was gracefully stopped,
    /// and the data storage was additionally checked for consistency.
    GracefullyStoppedChecked,
}

impl fmt::Display for RecoveryState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryState::Unknown => write!(f, "Unknown"),
            RecoveryState::Error => write!(f, "Error"),
            RecoveryState::Fresh => write!(f, "Fresh"),
            RecoveryState::MissingData => write!(f, "Missing Data"),
            RecoveryState::Finished => write!(f, "Finished"),
            RecoveryState::AbortedUnchecked => write!(f, "Aborted Unchecked"),
            RecoveryState::AbortedChecked => write!(f, "Aborted Checked"),
            RecoveryState::GracefullyStopped => write!(f, "Gracefully Stopped"),
            RecoveryState::GracefullyStoppedChecked => write!(f, "Gracefully Stopped Checked"),
        }
    }
}
