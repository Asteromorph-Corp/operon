use crate::meta_storage::{MetaClient, MetaStorageError};
use crate::schema_base::Job;

pub async fn get_ticket_summary<J: Job>(
    client: MetaClient<'_>,
) -> Result<(i64, i64, i64), MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let stmt = format!("SELECT * FROM {schema_prefix}ticket_summary WHERE job_id = $1");

    let row = client
        .query_opt(&stmt, &[&J::id()])
        .await?
        .ok_or(MetaStorageError::NotFound(format!(
            "Ticket summary for job {}",
            J::id()
        )))?;

    let done: i64 = row.get("done");
    let queued: i64 = row.get("queued");
    let waiting: i64 = row.get("waiting");
    Ok((done, queued, waiting))
}
