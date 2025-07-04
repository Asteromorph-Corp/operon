use crate::meta_storage::{MetaClient, MetaStorageError};

pub async fn init_ticket_summary(client: MetaClient<'_>) -> Result<(), MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let stmt = format!(
        "CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_summary (
            job_id TEXT NOT NULL,
            waiting BIGINT NOT NULL DEFAULT 0,
            queued BIGINT NOT NULL DEFAULT 0,
            done BIGINT NOT NULL DEFAULT 0,
            PRIMARY KEY (job_id)
            CHECK (
                waiting >= 0 AND
                queued >= 0 AND
                done >= 0
            )
        );

        CREATE OR REPLACE FUNCTION {schema_prefix}trg_ticket_summary() RETURNS TRIGGER AS $$
        BEGIN
            IF TG_OP = 'INSERT' THEN
                UPDATE {schema_prefix}ticket_summary s
                SET
                    waiting = waiting + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'waiting'),
                    queued  = queued  + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'queued'),
                    done    = done    + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'done')
                WHERE job_id = TG_ARGV[0];

            ELSIF TG_OP = 'UPDATE' THEN
                UPDATE {schema_prefix}ticket_summary
                SET
                    waiting = waiting 
                                - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'waiting')
                                + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'waiting'),
                    queued  = queued  
                                - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'queued')
                                + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'queued'),
                    done    = done    
                                - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'done')
                                + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'done')
                WHERE job_id = TG_ARGV[0];

            ELSIF TG_OP = 'DELETE' THEN
                UPDATE {schema_prefix}ticket_summary
                SET
                    waiting = waiting - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'waiting'),
                    queued  = queued  - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'queued'),
                    done    = done    - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'done')
                WHERE job_id = TG_ARGV[0];

            ELSIF TG_OP = 'TRUNCATE' THEN
                UPDATE {schema_prefix}ticket_summary
                SET waiting = 0, queued = 0, done = 0
                WHERE job_id = TG_ARGV[0];
            END IF;

            RETURN NULL;
        END;
        $$ LANGUAGE plpgsql;"
    );
    client.execute(&stmt, &[]).await?;
    Ok(())
}
