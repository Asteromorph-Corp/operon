use async_trait::async_trait;

use crate::{
    meta_storage::{ConnectionWithSchema, MetaClient, MetaContext, MetaStorageError},
    ui::UiStateUpdate,
};

#[async_trait]
pub trait MetaStorage: Send + Sync + 'static {
    fn new(context: MetaContext) -> Self;

    async fn conn(&self) -> Result<ConnectionWithSchema, MetaStorageError>; // `Return self.context.pool.get().await`
    async fn conn_static(&self) -> Result<ConnectionWithSchema<'static>, MetaStorageError>; // `Return self.context.pool.get().await`

    fn schema(&self) -> Option<&str>; // `Return self.context.schema.as_deref()`

    fn get_prefix(&self) -> String {
        match self.schema() {
            Some(s) => format!("{s}."),
            None => String::new(),
        }
    }

    async fn init(&self) -> Result<(), MetaStorageError> {
        let mut conn = self.conn().await?;
        let tx = conn.transaction().await?;
        self.init_schema(tx.as_client()).await?;
        self.init_footprint(tx.as_client()).await?;
        self.init_resolution(tx.as_client()).await?;
        self.init_tickets(tx.as_client()).await?;
        tx.commit().await?;
        Ok(())
    }

    /// If given, initialize the schema in the database.
    async fn init_schema(&self, client: MetaClient<'_>) -> Result<(), MetaStorageError> {
        let Some(schema) = self.schema() else {
            return Ok(());
        };
        let create_schema = format!("CREATE SCHEMA IF NOT EXISTS {schema}");
        client.execute(&create_schema, &[]).await?;
        Ok(())
    }

    // Footprint
    // Operations for footprinting the PSQL metadata storage.
    // Given a connection, these methods footprint the metadata storage.

    /// Initialize the footprint table.
    /// Note that this function is idempotent, i.e. calling it multiple times,
    /// or calling it on an already-initialized storage will do nothing.
    async fn init_footprint(&self, client: MetaClient<'_>) -> Result<(), MetaStorageError> {
        let schema_prefix = self.get_prefix();
        let stmt = format!(
            "CREATE TABLE IF NOT EXISTS {schema_prefix}footprint (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )"
        );
        client.execute(&stmt, &[]).await?;
        Ok(())
    }

    /// Clear the footprint table.
    async fn clear_footprint(&self, client: MetaClient<'_>) -> Result<(), MetaStorageError> {
        let schema_prefix = self.get_prefix();
        let stmt = format!("TRUNCATE TABLE {schema_prefix}footprint");
        client.execute(&stmt, &[]).await?;
        Ok(())
    }

    /// Get a footprint value by key.
    async fn get_footprint(
        &self,
        client: MetaClient<'_>,
        key: impl Into<String> + Send + Sync,
    ) -> Result<Option<String>, MetaStorageError> {
        let key: String = key.into();
        let schema_prefix = self.get_prefix();
        let stmt = format!("SELECT value FROM {schema_prefix}footprint WHERE key = $1");
        let row = client.query_opt(&stmt, &[&key]).await?;
        Ok(row.map(|r| r.get::<_, &str>(0).to_string()))
    }

    /// Set a footprint key-value pair.
    async fn put_footprint(
        &self,
        client: MetaClient<'_>,
        key: impl Into<String> + Send + Sync,
        value: impl Into<String> + Send + Sync,
    ) -> Result<(), MetaStorageError> {
        let key: String = key.into();
        let value: String = value.into();

        let schema_prefix = self.get_prefix();
        let stmt = format!(
            "INSERT INTO {schema_prefix}footprint (key, value)
            VALUES ($1, $2)
            ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
        );
        client.execute(&stmt, &[&key, &value]).await?;
        Ok(())
    }

    // Resolution
    // Operations for the PSQL fact storage.
    // Given a connection, these methods perform the necessaray operations on the fact storage.

    /// Initialize the PSQL fact storage.
    /// Note that this function is idempotent, i.e. calling it multiple times,
    /// or calling it on an already-initialized storage will do nothing.
    ///
    /// This function will `CREATE IF NOT EXISTS` the following tables under the given schema:
    ///
    /// * `dimension_i`:
    ///   ```sql
    ///   CREATE TABLE dimension_i (
    ///       i_ub BIGINT NOT NULL
    ///   )
    ///   ```
    /// * `dimension_j`:
    ///   ```sql
    ///   CREATE TABLE dimension_j (
    ///       i BIGINT,
    ///       j_ub BIGINT NOT NULL,
    ///       PRIMARY KEY (i)
    ///   )
    ///   ```
    /// * `dimension_k`:
    ///   ```sql
    ///   CREATE TABLE dimension_k (
    ///       i BIGINT,
    ///       k_ub BIGINT NOT NULL,
    ///       PRIMARY KEY (i)
    ///   )
    ///   ```
    async fn init_resolution(&self, client: MetaClient<'_>) -> Result<(), MetaStorageError>; // `facts_psql::init`, 847~

    /// Clear the data in the PSQL fact storage, assuming the tables are already initialized.
    async fn clear_resolution(&self, client: MetaClient<'_>) -> Result<(), MetaStorageError>; // `facts_psql::clear`, 877~

    /// Get the primary resolution from the PSQL fact storage.
    async fn get_primary_resolution(
        &self,
        client: MetaClient<'_>,
    ) -> Result<Option<usize>, MetaStorageError>; // `facts_psql::get_resolution`, 891~

    /// Put a primary resolution into the PSQL fact storage.
    async fn put_primary_resolution(
        &self,
        client: MetaClient<'_>,
        resolution: usize,
    ) -> Result<(), MetaStorageError>;

    // Ticket
    // Operations for the PSQL ticket storage.
    // Given a connection, these methods perform the necessary operations on the ticket storage.

    /// Initialize the PSQL ticket storage.
    /// Note that this function is idempotent, i.e. calling it multiple times,
    /// or calling it on an already-initialized storage will do nothing.
    ///
    /// This function will `CREATE IF NOT EXISTS` the necessary type and tables under the given schema:
    ///
    /// * `ticket_status`:
    /// ```sql
    /// CREATE TYPE ticket_status AS ENUM (
    ///     'waiting',
    ///     'queued',
    ///     'done',
    /// )
    /// ```
    /// * `ticket_beta`:
    /// ```sql
    /// CREATE TABLE ticket_beta (
    ///     i BIGINT,
    ///     resolved BOOLEAN NOT NULL,
    ///     deps_count BIGINT NOT NULL,
    ///     deps_quota BIGINT,
    ///     deps_done BOOLEAN NOT NULL,
    ///     status ticket_status NOT NULL,
    ///     PRIMARY KEY (i)
    /// )
    /// ```
    /// * `ticket_gamma`:
    /// ```sql
    /// CREATE TABLE ticket_gamma (
    ///     i BIGINT,
    ///     resolved BOOLEAN NOT NULL,
    ///     deps_count BIGINT NOT NULL,
    ///     deps_quota BIGINT,
    ///     deps_done BOOLEAN NOT NULL,
    ///     status ticket_status NOT NULL,
    ///     PRIMARY KEY (i)
    /// )
    /// ```
    /// * `ticket_delta`:
    /// ```sql
    /// CREATE TABLE ticket_delta (
    ///     i BIGINT,
    ///     j BIGINT,
    ///     k BIGINT,
    ///     resolved BOOLEAN NOT NULL,
    ///     deps_count BIGINT NOT NULL,
    ///     deps_quota BIGINT,
    ///     deps_done BOOLEAN NOT NULL,
    ///     status ticket_status NOT NULL,
    ///     PRIMARY KEY (i, j, k)
    /// )
    /// ```
    /// * `ticket_epsilon`:
    /// ```sql
    /// CREATE TABLE ticket_epsilon (
    ///     i BIGINT,
    ///     k BIGINT,
    ///     resolved BOOLEAN NOT NULL,
    ///     deps_count BIGINT NOT NULL,
    ///     deps_quota BIGINT,
    ///     deps_done BOOLEAN NOT NULL,
    ///     status ticket_status NOT NULL,
    ///     PRIMARY KEY (i, k)
    /// )
    /// ```
    /// * `ticket_zeta`:
    /// ```sql
    /// CREATE TABLE ticket_zeta (
    ///     i BIGINT,
    ///     resolved BOOLEAN NOT NULL,
    ///     deps_count BIGINT NOT NULL,
    ///     deps_quota BIGINT,
    ///     deps_done BOOLEAN NOT NULL,
    ///     status ticket_status NOT NULL,
    ///     PRIMARY KEY (i)
    /// )
    /// ```
    ///
    /// Also, each table will have an associated summary table
    /// that keeps track of the number of tickets per status, for example:
    /// * `ticket_beta_status`:
    /// ```sql
    /// CREATE TABLE ticket_beta_status (
    ///     waiting BIGINT NOT NULL,
    ///     queued BIGINT NOT NULL,
    ///     done BIGINT NOT NULL,
    ///     CHECK (
    ///         waiting >= 0 AND
    ///         queued >= 0 AND
    ///         done >= 0
    ///     )
    /// )
    /// ```
    async fn init_tickets(&self, client: MetaClient<'_>) -> Result<(), MetaStorageError>; // `tickets_psql::init`, 1079~

    /// Clear the data from the PSQL ticket storage, assuming the tables are already initialized.
    async fn clear_tickets(&self, client: MetaClient<'_>) -> Result<(), MetaStorageError>; // `tickets_psql::clear`, 1623~

    /// Put the default (fully unresolved) tickets into the PSQL ticket storage.
    async fn put_default_tickets(&self, client: MetaClient<'_>) -> Result<(), MetaStorageError>;

    async fn get_ui_updates(
        &self,
        client: MetaClient<'_>,
    ) -> Result<Vec<UiStateUpdate>, MetaStorageError>; // `Scheduler::update_ui_all`, 5900~
}
