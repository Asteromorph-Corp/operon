use crate::meta_storage::{MetaClient, MetaStorageError};
use crate::schema::{Job, JobMetadata, Ticket, TicketStatus};
use crate::utils::{SchemaPrefix, SqlParams};

/// Helper struct for building SQL queries related to tickets.
pub struct TicketQueryBuilder<'a, const N: usize> {
    client: &'a MetaClient<'a>,
    job_meta: JobMetadata<N>,
}

impl<'a> MetaClient<'a> {
    /// Helper method to create a `ResolutionQueryBuilder` for a ticket of given job.
    pub fn ticket<const N: usize>(&'a self, job_meta: JobMetadata<N>) -> TicketQueryBuilder<'a, N> {
        TicketQueryBuilder {
            client: self,
            job_meta,
        }
    }
}

impl<const N: usize> TicketQueryBuilder<'_, N> {
    /// Initializes the ticket table.
    pub async fn init(&self) -> Result<(), MetaStorageError> {
        let schema_prefix = self.client.schema_prefix();
        let init_stmt = InitTicketQuery(schema_prefix, self.job_meta);
        let summary_stmt = TicketSummaryInsertQuery(schema_prefix);
        let trigger_stmts = TicketSummaryTriggerQuery(schema_prefix, self.job_meta);

        self.client.execute_stmt(&init_stmt, &[]).await?;
        self.client
            .execute_stmt(&summary_stmt, &[&self.job_meta.id])
            .await?;
        self.client.batch_execute_stmt(&trigger_stmts).await?;
        Ok(())
    }

    /// Clears the ticket table.
    pub async fn clear(&self) -> Result<(), MetaStorageError> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = ClearTicketQuery(schema_prefix, self.job_meta);
        self.client.execute_stmt(&stmt, &[]).await?;
        Ok(())
    }

    /// Gets all tickets with a given status.
    pub async fn get_all(&self, status: TicketStatus) -> Result<Vec<Ticket<N>>, MetaStorageError> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = GetAllTicketQuery(schema_prefix, self.job_meta);
        let rows = self.client.query_stmt(&stmt, &[&status]).await?;
        let jobs = rows
            .iter()
            .map(|row| Ticket::from_sql_row(self.job_meta, row))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(jobs)
    }

    /// Puts a ticket into the table.
    pub async fn put(&self, ticket: Ticket<N>) -> Result<(), MetaStorageError> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = PutTicketQuery(schema_prefix, self.job_meta);
        let params = ticket.as_sql_params()?;
        self.client.execute_stmt(&stmt, &params.borrow()).await?;
        Ok(())
    }

    /// Marks the ticket corresponding to a given job as done.
    pub async fn mark_done(&self, job: Job<N>) -> Result<(), MetaStorageError> {
        let schema = self.client.schema_prefix();
        let stmt = MarkDoneQuery(schema, self.job_meta);
        let params = SqlParams::from_usize(job.coordinate)?;
        self.client.execute_stmt(&stmt, &params.borrow()).await?;
        Ok(())
    }

    pub async fn get_status(&self) -> Result<(i64, i64, i64), MetaStorageError> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = GetStatusQuery(schema_prefix);

        let job_id = self.job_meta.id;
        let Some(row) = self.client.query_opt_stmt(&stmt, &[&job_id]).await? else {
            return Err(MetaStorageError::NotFound(format!(
                "Ticket summary for job {job_id}",
            )));
        };

        let done: i64 = row.get("done");
        let queued: i64 = row.get("queued");
        let waiting: i64 = row.get("waiting");
        Ok((done, queued, waiting))
    }
}

/// Helper struct to generate the SQL query for initializing a ticket table.
struct InitTicketQuery<'a, const N: usize>(SchemaPrefix<'a>, JobMetadata<N>);

impl<const N: usize> std::fmt::Display for InitTicketQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let dims = self.1.dims;

        writeln!(f, "CREATE TABLE IF NOT EXISTS {schema}ticket_{id} (",)?;
        for dim in dims {
            writeln!(f, "    {dim} BIGINT,")?;
        }
        writeln!(f, "    deps_done BIGINT NOT NULL,")?;
        writeln!(f, "    deps_quota BIGINT NOT NULL,")?;
        write!(f, "    status {schema}ticket_status NOT NULL")?;

        if dims.is_empty() {
            writeln!(f)?;
            return write!(f, ");");
        }

        writeln!(f, ",")?;
        write!(f, "    PRIMARY KEY (")?;
        for (idx, dim) in dims.iter().enumerate() {
            if idx > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{dim}")?;
        }
        writeln!(f, ")")?;
        write!(f, ");")
    }
}

/// Helper struct to generate the SQL queries for ticket summary insert.
struct TicketSummaryInsertQuery<'a>(SchemaPrefix<'a>);

impl<'a> std::fmt::Display for TicketSummaryInsertQuery<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;

        writeln!(
            f,
            "INSERT INTO {schema}ticket_summary (job_id, waiting, queued, done)"
        )?;
        writeln!(f, "VALUES ($1, 0, 0, 0)")?;
        write!(f, "ON CONFLICT (job_id) DO NOTHING;")
    }
}

/// Helper struct to generate the SQL queries for ticket summary triggers.
struct TicketSummaryTriggerQuery<'a, const N: usize>(SchemaPrefix<'a>, JobMetadata<N>);

impl<const N: usize> std::fmt::Display for TicketSummaryTriggerQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;

        writeln!(f, "CREATE OR REPLACE TRIGGER ticket_{id}_summary_ins_trg")?;
        writeln!(f, "    AFTER INSERT ON {schema}ticket_{id}")?;
        writeln!(f, "    REFERENCING NEW TABLE AS NEW_TABLE")?;
        writeln!(f, "    FOR EACH STATEMENT")?;
        writeln!(
            f,
            "    EXECUTE FUNCTION {schema}trg_ticket_summary('{id}');"
        )?;
        writeln!(f)?;

        writeln!(f, "CREATE OR REPLACE TRIGGER ticket_{id}_summary_upd_trg")?;
        writeln!(f, "    AFTER UPDATE ON {schema}ticket_{id}")?;
        writeln!(f, "    REFERENCING")?;
        writeln!(f, "        NEW TABLE AS NEW_TABLE")?;
        writeln!(f, "        OLD TABLE AS OLD_TABLE")?;
        writeln!(f, "    FOR EACH STATEMENT")?;
        writeln!(
            f,
            "    EXECUTE FUNCTION {schema}trg_ticket_summary('{id}');"
        )?;

        writeln!(f)?;
        writeln!(f, "CREATE OR REPLACE TRIGGER ticket_{id}_summary_del_trg")?;
        writeln!(f, "    AFTER DELETE ON {schema}ticket_{id}")?;
        writeln!(f, "    REFERENCING OLD TABLE AS OLD_TABLE")?;
        writeln!(f, "    FOR EACH STATEMENT")?;
        writeln!(
            f,
            "    EXECUTE FUNCTION {schema}trg_ticket_summary('{id}');"
        )?;
        writeln!(f)?;

        writeln!(f, "CREATE OR REPLACE TRIGGER ticket_{id}_summary_trunc_trg")?;
        writeln!(f, "    AFTER TRUNCATE ON {schema}ticket_{id}")?;
        writeln!(f, "    FOR EACH STATEMENT")?;
        write!(
            f,
            "    EXECUTE FUNCTION {schema}trg_ticket_summary('{id}');"
        )
    }
}

/// Helper struct to generate the SQL query for clearing a ticket table.
struct ClearTicketQuery<'a, const N: usize>(SchemaPrefix<'a>, JobMetadata<N>);

impl<const N: usize> std::fmt::Display for ClearTicketQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;

        write!(f, "TRUNCATE TABLE {schema}ticket_{id};")
    }
}

/// Helper struct to generate the SQL query for getting all tickets for a given job.
struct GetAllTicketQuery<'a, const N: usize>(SchemaPrefix<'a>, JobMetadata<N>);

impl<const N: usize> std::fmt::Display for GetAllTicketQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;

        write!(f, "SELECT * FROM {schema}ticket_{id} WHERE status = $1;")
    }
}

/// Helper struct to generate the SQL query for inserting default tickets for a job.
struct PutTicketQuery<'a, const N: usize>(SchemaPrefix<'a>, JobMetadata<N>);

impl<const N: usize> std::fmt::Display for PutTicketQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let dims = self.1.dims;

        write!(f, "INSERT INTO {schema}ticket_{id} (")?;
        for dim in dims {
            write!(f, "{dim}, ")?;
        }
        writeln!(f, "deps_done, deps_quota, status)")?;

        write!(f, "VALUES (")?;
        for i in 1..=(dims.len() + 3) {
            if i != 1 {
                write!(f, ", ")?;
            }
            write!(f, "${i}")?;
        }
        writeln!(f, ")")?;

        write!(f, "ON CONFLICT DO NOTHING;")?;

        Ok(())
    }
}

/// An helper struct to generate the SQL query for marking a ticket as done for a given job.
struct MarkDoneQuery<'a, const N: usize>(SchemaPrefix<'a>, JobMetadata<N>);

impl<const N: usize> std::fmt::Display for MarkDoneQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let dims = self.1.dims;

        write!(f, "UPDATE {schema}ticket_{id} SET status = 'done'")?;
        for (idx, dim) in dims.iter().enumerate() {
            if idx == 0 {
                write!(f, " WHERE")?;
            } else {
                write!(f, " AND")?;
            }
            write!(f, " {dim} = ${}", idx + 1)?;
        }
        write!(f, ";")
    }
}

/// An helper struct to generate the SQL query for fetching the job status.
struct GetStatusQuery<'a>(SchemaPrefix<'a>);

impl<'a> std::fmt::Display for GetStatusQuery<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;

        write!(f, "SELECT * FROM {schema}ticket_summary WHERE job_id = $1;")
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::{fixture, rstest};

    use super::*;
    use crate::schema::JobMetadata;

    fn job_alpha() -> JobMetadata<0> {
        JobMetadata {
            id: "alpha",
            spawn_dim: Some("i"),
            dims: [],
        }
    }

    fn job_beta() -> JobMetadata<1> {
        JobMetadata {
            id: "beta",
            spawn_dim: Some("j"),
            dims: ["i"],
        }
    }

    #[fixture]
    fn schema_prefix() -> SchemaPrefix<'static> {
        SchemaPrefix(Some("test_meta"))
    }

    #[rstest]
    #[case::empty(
        job_alpha(),
        indoc! {"
            CREATE TABLE IF NOT EXISTS test_meta.ticket_alpha (
                deps_done BIGINT NOT NULL,
                deps_quota BIGINT NOT NULL,
                status test_meta.ticket_status NOT NULL
            );"
        }
    )]
    #[case::simple(
        job_beta(),
        indoc! {"
            CREATE TABLE IF NOT EXISTS test_meta.ticket_beta (
                i BIGINT,
                deps_done BIGINT NOT NULL,
                deps_quota BIGINT NOT NULL,
                status test_meta.ticket_status NOT NULL,
                PRIMARY KEY (i)
            );"
        },
    )]
    fn test_init_ticket_query<const N: usize>(
        schema_prefix: SchemaPrefix<'static>,
        #[case] job: JobMetadata<N>,
        #[case] expected: &str,
    ) {
        let stmt = InitTicketQuery(schema_prefix, job).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(
        indoc! {"
            INSERT INTO test_meta.ticket_summary (job_id, waiting, queued, done)
            VALUES ($1, 0, 0, 0)
            ON CONFLICT (job_id) DO NOTHING;"
        },
    )]
    fn test_ticket_summary_insert_query(
        schema_prefix: SchemaPrefix<'static>,
        #[case] expected: &str,
    ) {
        let stmt = TicketSummaryInsertQuery(schema_prefix).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(
        job_beta(),
        indoc! {"
            CREATE OR REPLACE TRIGGER ticket_beta_summary_ins_trg
                AFTER INSERT ON test_meta.ticket_beta
                REFERENCING NEW TABLE AS NEW_TABLE
                FOR EACH STATEMENT
                EXECUTE FUNCTION test_meta.trg_ticket_summary('beta');

            CREATE OR REPLACE TRIGGER ticket_beta_summary_upd_trg
                AFTER UPDATE ON test_meta.ticket_beta
                REFERENCING
                    NEW TABLE AS NEW_TABLE
                    OLD TABLE AS OLD_TABLE
                FOR EACH STATEMENT
                EXECUTE FUNCTION test_meta.trg_ticket_summary('beta');

            CREATE OR REPLACE TRIGGER ticket_beta_summary_del_trg
                AFTER DELETE ON test_meta.ticket_beta
                REFERENCING OLD TABLE AS OLD_TABLE
                FOR EACH STATEMENT
                EXECUTE FUNCTION test_meta.trg_ticket_summary('beta');

            CREATE OR REPLACE TRIGGER ticket_beta_summary_trunc_trg
                AFTER TRUNCATE ON test_meta.ticket_beta
                FOR EACH STATEMENT
                EXECUTE FUNCTION test_meta.trg_ticket_summary('beta');"
        }
    )]
    fn test_ticket_summary_trigger_query<const N: usize>(
        schema_prefix: SchemaPrefix<'static>,
        #[case] job: JobMetadata<N>,
        #[case] expected: &str,
    ) {
        let stmt = TicketSummaryTriggerQuery(schema_prefix, job).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(job_beta(), "TRUNCATE TABLE test_meta.ticket_beta;")]
    fn test_clear_ticket_query<const N: usize>(
        schema_prefix: SchemaPrefix<'static>,
        #[case] job: JobMetadata<N>,
        #[case] expected: &str,
    ) {
        let stmt = ClearTicketQuery(schema_prefix, job).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(job_beta(), "SELECT * FROM test_meta.ticket_beta WHERE status = $1;")]
    fn test_get_all_ticket_query<const N: usize>(
        schema_prefix: SchemaPrefix<'static>,
        #[case] job: JobMetadata<N>,
        #[case] expected: &str,
    ) {
        let stmt = GetAllTicketQuery(schema_prefix, job).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::empty(
        job_alpha(),
        indoc! {"
            INSERT INTO test_meta.ticket_alpha (deps_done, deps_quota, status)
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING;"
        }
    )]
    #[case::simple(
        job_beta(),
        indoc! {"
            INSERT INTO test_meta.ticket_beta (i, deps_done, deps_quota, status)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT DO NOTHING;"
        }
    )]
    fn test_put_ticket_query<const N: usize>(
        schema_prefix: SchemaPrefix<'static>,
        #[case] job: JobMetadata<N>,
        #[case] expected: &str,
    ) {
        let stmt = PutTicketQuery(schema_prefix, job).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::empty(job_alpha(), "UPDATE test_meta.ticket_alpha SET status = 'done';")]
    #[case::simple(
        job_beta(),
        "UPDATE test_meta.ticket_beta SET status = 'done' WHERE i = $1;"
    )]
    fn test_mark_done_query<const N: usize>(
        schema_prefix: SchemaPrefix<'static>,
        #[case] job: JobMetadata<N>,
        #[case] expected: &str,
    ) {
        let stmt = MarkDoneQuery(schema_prefix, job).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple("SELECT * FROM test_meta.ticket_summary WHERE job_id = $1;")]
    fn test_get_status_query(schema_prefix: SchemaPrefix<'static>, #[case] expected: &str) {
        let stmt = GetStatusQuery(schema_prefix).to_string();
        assert_eq!(stmt, expected);
    }
}
