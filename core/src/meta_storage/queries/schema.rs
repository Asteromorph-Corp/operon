use crate::meta_storage::{MetaClient, MetaStorageError};

impl MetaClient<'_> {
    /// If the meta storage schema is specified, initialize the schema in the database.
    pub async fn init_schema(&self) -> Result<(), MetaStorageError> {
        let Some(schema) = self.schema() else {
            return Ok(());
        };
        let create_schema = format!("CREATE SCHEMA IF NOT EXISTS {schema}");
        self.execute(&create_schema, &[]).await?;
        Ok(())
    }

    pub async fn init_ticket_status_type(&self) -> Result<(), MetaStorageError> {
        let ticket_status_type = self.ticket_status_type();
        let create_status_type = format!(
            "DO $$ BEGIN
                CREATE TYPE {ticket_status_type} AS ENUM (
                    'waiting',
                    'queued',
                    'done'
                );
            EXCEPTION
                WHEN duplicate_object THEN null;
            END $$;"
        );
        self.execute(&create_status_type, &[]).await?;
        Ok(())
    }
}
