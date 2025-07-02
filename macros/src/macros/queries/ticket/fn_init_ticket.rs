use quote::quote;

use crate::{
    configs::JobConfig,
    utils::{init_ticket_ident, operon_ident},
};

/// Helper struct to generate the SQL query for initializing a ticket table.
struct InitTicketQuery<'a>(&'a JobConfig);

impl std::fmt::Display for InitTicketQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "CREATE TABLE IF NOT EXISTS {{schema_prefix}}ticket_{} (",
            self.0.id
        )?;
        for dim in &self.0.dims {
            writeln!(f, "    {dim} BIGINT,")?;
        }
        writeln!(f, "    resolved BOOLEAN NOT NULL,")?;
        writeln!(f, "    deps_count BIGINT NOT NULL,")?;
        writeln!(f, "    deps_quota BIGINT,")?;
        writeln!(f, "    deps_done BOOLEAN NOT NULL,")?;
        writeln!(f, "    status {{ticket_status_type}} NOT NULL,")?;
        writeln!(f, "    PRIMARY KEY (i)")?;
        write!(f, ");")
    }
}

/// Helper struct to generate the SQL query for ticket summary triggers.
struct TicketSummaryQuery<'a>(&'a JobConfig);

impl std::fmt::Display for TicketSummaryQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id = &self.0.id;

        writeln!(
            f,
            "INSERT INTO {{schema_prefix}}ticket_summary (job_id, waiting, queued, done)"
        )?;
        writeln!(f, "VALUES ($1, 0, 0, 0)")?;
        writeln!(f, "ON CONFLICT DO NOTHING;")?;
        writeln!(f)?;

        writeln!(f, "CREATE OR REPLACE TRIGGER ticket_{id}_summary_ins_trg")?;
        writeln!(f, "    AFTER INSERT ON {{schema_prefix}}ticket_{id}")?;
        writeln!(f, "    REFERENCING NEW TABLE AS NEW_TABLE")?;
        writeln!(f, "    FOR EACH STATEMENT")?;
        writeln!(
            f,
            "    EXECUTE FUNCTION {{schema_prefix}}trg_ticket_summary('{id}');"
        )?;
        writeln!(f)?;

        writeln!(f, "CREATE OR REPLACE TRIGGER ticket_{id}_summary_upd_trg")?;
        writeln!(f, "    AFTER UPDATE ON {{schema_prefix}}ticket_{id}")?;
        writeln!(f, "    REFERENCING")?;
        writeln!(f, "        NEW TABLE AS NEW_TABLE")?;
        writeln!(f, "        OLD TABLE AS OLD_TABLE")?;
        writeln!(f, "    FOR EACH STATEMENT")?;
        writeln!(
            f,
            "    EXECUTE FUNCTION {{schema_prefix}}trg_ticket_summary('{id}');"
        )?;

        writeln!(f)?;
        writeln!(f, "CREATE OR REPLACE TRIGGER ticket_{id}_summary_del_trg")?;
        writeln!(f, "    AFTER DELETE ON {{schema_prefix}}ticket_{id}")?;
        writeln!(f, "    REFERENCING OLD TABLE AS OLD_TABLE")?;
        writeln!(f, "    FOR EACH STATEMENT")?;
        writeln!(
            f,
            "    EXECUTE FUNCTION {{schema_prefix}}trg_ticket_summary('{id}');"
        )?;
        writeln!(f)?;

        writeln!(f, "CREATE OR REPLACE TRIGGER ticket_{id}_summary_trunc_trg")?;
        writeln!(f, "    AFTER TRUNCATE ON {{schema_prefix}}ticket_{id}")?;
        writeln!(f, "    FOR EACH STATEMENT")?;
        write!(
            f,
            "    EXECUTE FUNCTION {{schema_prefix}}trg_ticket_summary('{id}');"
        )
    }
}

/// Generates the `init_ticket_*` function for a given job.
///
/// Example:
/// ```ignore
/// pub async fn init_tickets_beta(
///     client: operon::meta_storage::MetaClient<'_>
/// ) -> Result<(), operon::meta_storage::MetaStorageError> {
///     let schema_prefix = client.schema_prefix();
///     let ticket_status_type = client.ticket_status_type();
///     let create_table = format!(
///         "CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_beta (
///             i BIGINT,
///             resolved BOOLEAN NOT NULL,
///             deps_count BIGINT NOT NULL,
///             deps_quota BIGINT,
///             deps_done BOOLEAN NOT NULL,
///             status {ticket_status_type} NOT NULL,
///             PRIMARY KEY (i)
///         )"
///     ); // TODO: move this to `Ticket` trait and automate using derive macro
///     let init_summary = format!(
///         "INSERT INTO {schema_prefix}ticket_summary (job_id, waiting, queued, done)
///         VALUES ($1, 0, 0, 0)
///         ON CONFLICT DO NOTHING;
///
///         CREATE OR REPLACE TRIGGER ticket_beta_summary_ins_trg
///             AFTER INSERT ON {schema_prefix}ticket_beta
///             REFERENCING NEW TABLE AS NEW_TABLE
///             FOR EACH STATEMENT
///             EXECUTE FUNCTION {schema_prefix}trg_ticket_summary('beta');
///
///         CREATE OR REPLACE TRIGGER ticket_beta_summary_upd_trg
///             AFTER UPDATE ON {schema_prefix}ticket_beta
///             REFERENCING
///                 NEW TABLE AS NEW_TABLE
///                 OLD TABLE AS OLD_TABLE
///             FOR EACH STATEMENT
///             EXECUTE FUNCTION {schema_prefix}trg_ticket_summary('beta');
///
///         CREATE OR REPLACE TRIGGER ticket_beta_summary_del_trg
///             AFTER DELETE ON {schema_prefix}ticket_beta
///             REFERENCING
///                 OLD TABLE AS OLD_TABLE
///             FOR EACH STATEMENT
///             EXECUTE FUNCTION {schema_prefix}trg_ticket_summary('beta');
///
///         CREATE OR REPLACE TRIGGER ticket_beta_summary_trunc_trg
///             AFTER TRUNCATE ON {schema_prefix}ticket_beta
///             FOR EACH STATEMENT
///             EXECUTE FUNCTION {schema_prefix}trg_ticket_summary('beta');"
///     );
///     client.execute(&create_table, &[]).await?;
///     client.execute(&init_summary, &[&BETA_ID]).await?;
///     Ok(())
/// }
/// ```
pub(super) fn fn_init_ticket(job: &JobConfig) -> proc_macro2::TokenStream {
    let operon = operon_ident();
    let fn_ident = init_ticket_ident(&job.id);
    let init_ticket_query = InitTicketQuery(job).to_string();
    let ticket_summary_query = TicketSummaryQuery(job).to_string();

    quote! {
        pub async fn #fn_ident(
            client: #operon::meta_storage::MetaClient<'_>,
        ) -> Result<(), #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let ticket_status_type = client.ticket_status_type();
            let init_stmt = format!(#init_ticket_query);
            let summary_stmt = format!(#ticket_summary_query);

            client.execute(init_stmt, &[]).await?;
            client.execute(&summary_stmt, &[&job.id]).await?;

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    #[test]
    fn test_init_ticket_query() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec!["a".to_string()],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
        };

        let init_ticket = InitTicketQuery(&job).to_string();
        let expected = indoc::indoc! {"
            CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_beta (
                i BIGINT,
                resolved BOOLEAN NOT NULL,
                deps_count BIGINT NOT NULL,
                deps_quota BIGINT,
                deps_done BOOLEAN NOT NULL,
                status {ticket_status_type} NOT NULL,
                PRIMARY KEY (i)
            );"
        };

        assert_eq!(init_ticket, expected);
    }

    #[test]
    fn test_ticket_summary_query() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec!["a".to_string()],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
        };

        let ticket_summary = TicketSummaryQuery(&job).to_string();
        let expected = indoc::indoc! {"
            INSERT INTO {schema_prefix}ticket_summary (job_id, waiting, queued, done)
            VALUES ($1, 0, 0, 0)
            ON CONFLICT DO NOTHING;

            CREATE OR REPLACE TRIGGER ticket_beta_summary_ins_trg
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
                EXECUTE FUNCTION {schema_prefix}trg_ticket_summary('beta');
        "};

        assert_eq!(ticket_summary, expected);
    }

    #[test]
    fn test_fn_init_ticket() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec!["a".to_string()],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
        };

        let result = fn_init_ticket(&job).to_string();
        let init_stmt = indoc! {"
            CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_beta (
                i BIGINT,
                resolved BOOLEAN NOT NULL,
                deps_count BIGINT NOT NULL,
                deps_quota BIGINT,
                deps_done BOOLEAN NOT NULL,
                status {ticket_status_type} NOT NULL,
                PRIMARY KEY (i)
            );"
        };
        let summary_stmt = indoc! {"
            INSERT INTO {schema_prefix}ticket_summary (job_id, waiting, queued, done)
            VALUES ($1, 0, 0, 0)
            ON CONFLICT DO NOTHING;

            CREATE OR REPLACE TRIGGER ticket_beta_summary_ins_trg
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
        };

        let expected = quote! {
            pub async fn init_ticket_beta(
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                let schema_prefix = client.schema_prefix();
                let ticket_status_type = client.ticket_status_type();
                let init_stmt = format!(#init_stmt);
                let summary_stmt = format!(#summary_stmt);

                client.execute(init_stmt, &[]).await?;
                client.execute(&summary_stmt, &[&job.id]).await?;

                Ok(())
            }
        };

        assert_eq!(result.to_string(), expected.to_string());
    }
}
