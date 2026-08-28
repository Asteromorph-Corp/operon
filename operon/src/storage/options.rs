use crate::storage::psql::PsqlStorageOptions;

/// Former name of [`PsqlStorageOptions`], kept as an alias for backward compatibility.
#[deprecated(note = "renamed to `PsqlStorageOptions`")]
pub type StorageOptions = PsqlStorageOptions;
