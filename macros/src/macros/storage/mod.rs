mod data_storage_definition;

mod impl_new;
mod impl_service_storage;
mod impl_storage;

mod batch_gets;
mod single_ops;

mod generic_constraints;
use generic_constraints::*;

mod mod_storage;
pub use mod_storage::*;
