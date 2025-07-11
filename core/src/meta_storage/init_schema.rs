use crate::meta_storage::{MetaClient, MetaStorageError};

/// If given, initialize the schema in the database.
pub async fn init_schema(client: MetaClient<'_>) -> Result<(), MetaStorageError> {
    let Some(schema) = client.schema() else {
        return Ok(());
    };
    let create_schema = format!("CREATE SCHEMA IF NOT EXISTS {schema}");
    client.execute(&create_schema, &[]).await?;
    Ok(())
}

pub async fn init_ticket_status_type(client: MetaClient<'_>) -> Result<(), MetaStorageError> {
    let ticket_status_type = client.ticket_status_type();
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
    client.execute(&create_status_type, &[]).await?;
    Ok(())
}
