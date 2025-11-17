//! Operations for footprinting the PSQL metadata storage.
//! Given a connection with an optional schema, this module provides operations footprint the
//! metadata storage.

use crate::meta_storage::{MetaClient, MetaStorageError};

impl MetaClient<'_> {
    /// Initialize the footprint table.
    /// Note that this function is idempotent, i.e. calling it multiple times,
    /// or calling it on an already-initialized storage will do nothing.
    pub async fn init_footprint(&self) -> Result<(), MetaStorageError> {
        let schema_prefix = self.schema_prefix();

        let stmt = format!(
            "CREATE TABLE IF NOT EXISTS {schema_prefix}footprint (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )"
        );
        self.execute(&stmt, &[]).await?;
        Ok(())
    }

    /// Get a footprint value by key.
    pub async fn get_footprint(
        &self,
        key: &'static str,
    ) -> Result<Option<String>, MetaStorageError> {
        let schema_prefix = self.schema_prefix();

        let stmt = format!("SELECT value FROM {schema_prefix}footprint WHERE key = $1");
        let row = self.query_opt(&stmt, &[&key]).await?;
        Ok(row.map(|r| r.get::<_, &str>(0).to_string()))
    }

    /// Set a footprint key-value pair.
    pub async fn put_footprint(
        &self,
        key: &'static str,
        value: String,
    ) -> Result<(), MetaStorageError> {
        let schema_prefix = self.schema_prefix();

        let stmt = format!(
            "INSERT INTO {schema_prefix}footprint (key, value)
            VALUES ($1, $2)
            ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
        );
        self.execute(&stmt, &[&key, &value]).await?;
        Ok(())
    }

    /// Clear the footprint table.
    pub async fn clear_footprint(&self) -> Result<(), MetaStorageError> {
        let schema_prefix = self.schema_prefix();

        let stmt = format!("TRUNCATE TABLE {schema_prefix}footprint");
        self.execute(&stmt, &[]).await?;
        Ok(())
    }
}
