//! Helpers shared by the metadata backend test modules.

use crate::meta_storage::psql::{PsqlMetaStorage, PsqlMetaStorageOptions};

/// The Postgres backend over the given schema, or `None` when no database is configured.
pub(super) fn psql_backend(schema: &str) -> Option<PsqlMetaStorage> {
    let uri = std::env::var("POSTGRES_URI").ok()?;
    let options = PsqlMetaStorageOptions::new(uri).with_schema(schema);
    Some(options.build().expect("build the Postgres backend"))
}
