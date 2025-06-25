use async_trait::async_trait;
use deadpool_postgres::Object;

use crate::{
    dimension::{JobEnum, ResolutionEnum},
    meta_storage::{MetaClient, MetaContext, MetaStorageError},
    ui::UiStateUpdate,
};

#[async_trait]
pub trait MetaStorage: Send + Sync + 'static {
    type JobEnum: JobEnum;
    type ResolutionEnum: ResolutionEnum;
    type ResolutionRequest: Default + Send + Sync;

    fn new(context: MetaContext) -> Self;

    async fn client(&self) -> Result<Object, MetaStorageError>; // `Return self.context.pool.get().await`
    fn schema(&self) -> Option<&str>; // `Return self.context.schema.as_deref()`

    fn get_prefix(&self) -> String {
        match self.schema() {
            Some(s) => format!("{s}."),
            None => String::new(),
        }
    }

    async fn init(&self) -> Result<(), MetaStorageError> {
        let mut client = self.client().await?;
        let tx = client.transaction().await?;
        self.init_schema(&tx).await?;
        self.init_footprint(&tx).await?;
        self.init_resolution(&tx).await?;
        self.init_tickets(&tx).await?;
        tx.commit().await?;
        Ok(())
    }

    /// If given, initialize the schema in the database.
    async fn init_schema(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError> {
        let Some(schema) = self.schema() else {
            return Ok(());
        };
        let conn: MetaClient<'_> = conn.into();
        let create_schema = format!("CREATE SCHEMA IF NOT EXISTS {schema}");
        conn.execute(&create_schema, &[]).await?;
        Ok(())
    }

    // Footprint
    // Operations for footprinting the PSQL metadata storage.
    // Given a connection, these methods footprint the metadata storage.

    /// Initialize the footprint table.
    /// Note that this function is idempotent, i.e. calling it multiple times,
    /// or calling it on an already-initialized storage will do nothing.
    async fn init_footprint(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError> {
        let conn: MetaClient<'_> = conn.into();
        let schema_prefix = self.get_prefix();
        let stmt = format!(
            "CREATE TABLE IF NOT EXISTS {schema_prefix}footprint (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )"
        );
        conn.execute(&stmt, &[]).await?;
        Ok(())
    }

    /// Clear the footprint table.
    async fn clear_footprint(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError> {
        let conn: MetaClient<'_> = conn.into();
        let schema_prefix = self.get_prefix();
        let stmt = format!("TRUNCATE TABLE {schema_prefix}footprint");
        conn.execute(&stmt, &[]).await?;
        Ok(())
    }

    /// Get a footprint value by key.
    async fn get_footprint(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
        key: impl Into<String> + Send + Sync,
    ) -> Result<Option<String>, MetaStorageError> {
        let conn: MetaClient<'_> = conn.into();
        let key: String = key.into();
        let schema_prefix = self.get_prefix();
        let stmt = format!("SELECT value FROM {schema_prefix}footprint WHERE key = $1");
        let row = conn.query_opt(&stmt, &[&key]).await?;
        Ok(row.map(|r| r.get::<_, &str>(0).to_string()))
    }

    /// Set a footprint key-value pair.
    async fn put_footprint(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
        key: impl Into<String> + Send + Sync,
        value: impl Into<String> + Send + Sync,
    ) -> Result<(), MetaStorageError> {
        let conn: MetaClient<'_> = conn.into();
        let key: String = key.into();
        let value: String = value.into();

        let schema_prefix = self.get_prefix();
        let stmt = format!(
            "INSERT INTO {schema_prefix}footprint (key, value)
            VALUES ($1, $2)
            ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
        );
        conn.execute(&stmt, &[&key, &value]).await?;
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
    async fn init_resolution(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError>; // `facts_psql::init`, 847~

    /// Clear the data in the PSQL fact storage, assuming the tables are already initialized.
    async fn clear_resolution(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError>; // `facts_psql::clear`, 877~

    /// Get the resolution for a given masked dimension request.
    async fn get_resolution(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
        request: &Self::ResolutionRequest,
    ) -> Result<Option<Self::ResolutionEnum>, MetaStorageError>; // `facts_psql::get_resolution`, 891~

    /// Put a resolution into the storage.
    async fn put_resolution(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
        resolution: &Self::ResolutionEnum,
    ) -> Result<(), MetaStorageError>; // `facts_psql::put_resolution`, 929~

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
    async fn init_tickets(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError>; // `tickets_psql::init`, 1079~

    /// Clear the data from the PSQL ticket storage, assuming the tables are already initialized.
    async fn clear_tickets(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError>; // `tickets_psql::clear`, 1623~

    /// Put the default (fully unresolved) tickets into the PSQL ticket storage.
    async fn put_default_tickets(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<(), MetaStorageError>;

    async fn get_ui_updates(
        &self,
        conn: impl Into<MetaClient<'_>> + Send + Sync,
    ) -> Result<Vec<UiStateUpdate>, MetaStorageError>; // `Scheduler::update_ui_all`, 5900~
}
