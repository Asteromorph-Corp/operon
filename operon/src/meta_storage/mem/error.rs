use std::convert::Infallible;

use crate::meta_storage::MetaResult;

/// A metadata result over the in-memory backend's error.
///
/// The in-memory backend has no transport layer, so every failure it can raise is a
/// backend-neutral domain error; `Infallible` leaves
/// [`MetaStorageError::Backend`](crate::meta_storage::MetaStorageError::Backend) unconstructable.
pub(super) type MemResult<T> = MetaResult<T, Infallible>;

/// The message used when a store lock is poisoned by a panic in another thread.
pub(super) const POISONED: &str = "in-memory metadata store lock poisoned";
