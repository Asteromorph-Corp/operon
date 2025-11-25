use crate::storage::StorageError;
use crate::storage::psql::StorageClient;

impl StorageClient<'_> {
    /// Initializes the footprint table.
    pub async fn init_footprint(&self) -> Result<(), StorageError> {
        let schema_prefix = self.schema_prefix();

        let stmt = format!(
            "CREATE TABLE IF NOT EXISTS {schema_prefix}_data_footprint (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )"
        );
        self.execute(&stmt, &[]).await?;
        Ok(())
    }

    /// Clears the footprint table.
    pub async fn clear_footprint(&self) -> Result<(), StorageError> {
        let schema_prefix = self.schema_prefix();
        let stmt = format!("TRUNCATE TABLE {schema_prefix}_data_footprint");
        self.execute(&stmt, &[]).await?;
        Ok(())
    }

    /// Gets a footprint value by key.
    pub async fn get_footprint(&self, key: &'static str) -> Result<Option<String>, StorageError> {
        let schema_prefix = self.schema_prefix();

        let stmt = format!("SELECT value FROM {schema_prefix}_data_footprint WHERE key = $1");
        let row = self.query_opt(&stmt, &[&key]).await?;
        Ok(row.map(|r| r.get::<_, &str>(0).to_string()))
    }

    /// Sets a footprint key-value pair.
    pub async fn put_footprint(
        &self,
        key: &'static str,
        value: String,
    ) -> Result<(), StorageError> {
        let schema_prefix = self.schema_prefix();

        let stmt = format!(
            "INSERT INTO {schema_prefix}_data_footprint (key, value)
            VALUES ($1, $2)
            ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
        );
        self.execute(&stmt, &[&key, &value]).await?;
        Ok(())
    }
}
