pub async fn init_ticket_beta(
    client: operon::meta_storage::MetaClient<'_>,
) -> Result<(), operon::meta_storage::MetaStorageError> {
    let schema_prefix = client.schema_prefix();
    let ticket_status_type = client.ticket_status_type();
    let init_stmt = format!(
        "CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_beta (
    i BIGINT,
    resolved BOOLEAN NOT NULL,
    deps_count BIGINT NOT NULL,
    deps_quota BIGINT,
    deps_done BOOLEAN NOT NULL,
    status {ticket_status_type} NOT NULL,
    PRIMARY KEY (i)
);"
    );
    let summary_stmt = format!(
        "INSERT INTO {schema_prefix}ticket_summary (job_id, waiting, queued, done)
VALUES ($1, 0, 0, 0)
ON CONFLICT DO NOTHING;"
    );
    let trigger_stmts = format!(
        "CREATE OR REPLACE TRIGGER ticket_beta_summary_ins_trg
    AFTER INSERT ON {schema_prefix}ticket_beta
    REFERENCING NEW TABLE AS NEW_TABLE
    FOR EACH STATEMENT
    EXECUTE FUNCTION {schema_prefix}trg_ticket_summary('beta');

CREATE OR REPLACE TRIGGER ticket_beta_summary_upd_trg
    AFTER UPDATE ON {schema_prefix}ticket_beta
    REFERENCING
        NEW TABLE AS NEW_TABLE
        OLD TABLE AS OLD_TABLE
    FOR EACH STATEMENT
    EXECUTE FUNCTION {schema_prefix}trg_ticket_summary('beta');

CREATE OR REPLACE TRIGGER ticket_beta_summary_del_trg
    AFTER DELETE ON {schema_prefix}ticket_beta
    REFERENCING OLD TABLE AS OLD_TABLE
    FOR EACH STATEMENT
    EXECUTE FUNCTION {schema_prefix}trg_ticket_summary('beta');

CREATE OR REPLACE TRIGGER ticket_beta_summary_trunc_trg
    AFTER TRUNCATE ON {schema_prefix}ticket_beta
    FOR EACH STATEMENT
    EXECUTE FUNCTION {schema_prefix}trg_ticket_summary('beta');"
    );

    client.execute(&init_stmt, &[]).await?;
    client
        .execute(
            &summary_stmt,
            &[&<schema::BetaJob as operon::schema_base::Job>::id()],
        )
        .await?;
    client.batch_execute(&trigger_stmts).await?;

    Ok(())
}
