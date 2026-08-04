use std::num::TryFromIntError;

use bytes::Bytes;
use futures::SinkExt;
use tokio_postgres::Row;

use crate::meta_storage::psql::error::PsqlResult;
use crate::meta_storage::psql::{PsqlClient, PsqlMetaError};
use crate::meta_storage::{MetaStorageError, MetaTicketApi};
use crate::schema::{
    DimensionMetadata, Job, JobMetadata, OptionCoordinate, Resolution, TableShape, Ticket,
    TicketStatus,
};
use crate::utils::{
    SchemaPrefix, SqlParams, box_sql, recorded_hash_query, replace_if_updated, table_shape,
};

/// Postgres wire (de)serialization for [`Ticket`], alongside the query builders that use it.
///
/// [`TicketStatus`] carries its own wire mapping via its `ToSql`/`FromSql` derive.
impl<const N: usize> Ticket<N> {
    /// Serializes a ticket into the ordered parameter list expected by the ticket table.
    fn as_sql_params(&self) -> Result<SqlParams, TryFromIntError> {
        let params = self
            .coordinate
            .iter()
            .map(|c| c.as_sql_param().map(box_sql))
            .chain([
                i64::try_from(self.deps_done()).map(box_sql),
                i64::try_from(self.deps_quota()).map(box_sql),
                Ok(box_sql(self.status)),
            ])
            .collect::<Result<Vec<_>, _>>()?;

        Ok(SqlParams::new(params))
    }

    /// Serializes a ticket into a `COPY ... FROM STDIN` CSV row.
    fn to_copy_string(self) -> Result<String, TryFromIntError> {
        Ok(self.as_sql_params()?.to_copy_string())
    }

    /// Materializes a ticket from a row of the ticket table.
    fn from_row(meta: JobMetadata<N>, row: &Row) -> Result<Ticket<N>, TryFromIntError> {
        let mut coordinate = [OptionCoordinate::none(); N];
        let mut i = 0;

        while i < N {
            let c = OptionCoordinate::from_sql_value(row.get(meta.dims[i]))?;
            coordinate[i] = c;
            i += 1;
        }

        let deps_done = usize::try_from(row.get::<_, i64>("deps_done"))?;
        let deps_quota = usize::try_from(row.get::<_, i64>("deps_quota"))?;
        let status = row.get::<_, TicketStatus>("status");

        Ok(Ticket::from_parts(
            coordinate, deps_done, deps_quota, status,
        ))
    }
}

/// Helper struct for building SQL queries related to tickets.
pub struct PsqlTicketQueryBuilder<'a, const N: usize> {
    client: &'a PsqlClient<'a>,
    job_meta: JobMetadata<N>,
}

impl<'a> PsqlClient<'a> {
    /// Helper method to create a `PsqlTicketQueryBuilder` for a ticket of given job.
    pub fn ticket<const N: usize>(
        &'a self,
        job_meta: JobMetadata<N>,
    ) -> PsqlTicketQueryBuilder<'a, N> {
        PsqlTicketQueryBuilder {
            client: self,
            job_meta,
        }
    }
}

impl<const N: usize> MetaTicketApi<N> for PsqlTicketQueryBuilder<'_, N> {
    type Error = PsqlMetaError;

    async fn shape(&self) -> PsqlResult<TableShape> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = recorded_hash_query(self.job_meta.id, schema_prefix, "_ticket_hash");
        let recorded = self.client.query_opt(&stmt, &[]).await?;
        Ok(table_shape(
            recorded.as_ref().map(|row| row.get("hash")),
            &self.job_meta,
        ))
    }

    /// Initializes the ticket table and its summary.
    async fn init(&self) -> PsqlResult<()> {
        let schema_prefix = self.client.schema_prefix();
        let id = self.job_meta.id;

        let init_stmt = InitTicketQuery(schema_prefix, self.job_meta);
        let trigger_stmts = TicketSummaryTriggerQuery(schema_prefix, self.job_meta);
        let delete_stmt = TicketSummaryDeleteQuery(schema_prefix, self.job_meta);
        let stmt = replace_if_updated(
            id,
            &[&format!("ticket_{id}")],
            &self.job_meta,
            schema_prefix,
            "_ticket_hash",
            format!("{init_stmt}\n{trigger_stmts}\n{delete_stmt}"),
        );

        let summary_stmt = TicketSummaryInsertQuery(schema_prefix);

        self.client.execute(&stmt, &[]).await?;
        self.client
            .execute_stmt(&summary_stmt, &[&self.job_meta.id])
            .await?;
        Ok(())
    }

    /// Clears the ticket table.
    async fn clear(&self) -> PsqlResult<()> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = ClearTicketQuery(schema_prefix, self.job_meta);
        self.client.execute_stmt(&stmt, &[]).await?;
        Ok(())
    }

    /// Gets all tickets with a given status.
    async fn get_all(&self, status: TicketStatus) -> PsqlResult<Vec<Ticket<N>>> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = GetAllTicketQuery(schema_prefix, self.job_meta);
        let rows = self.client.query_stmt(&stmt, &[&status]).await?;
        let tickets = rows
            .iter()
            .map(|row| Ticket::from_row(self.job_meta, row))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tickets)
    }

    /// Puts a ticket into the table.
    async fn put(&self, ticket: Ticket<N>) -> PsqlResult<()> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = PutTicketQuery(schema_prefix, self.job_meta);
        let params = ticket.as_sql_params()?;
        self.client.execute_stmt(&stmt, &params.borrow()).await?;
        Ok(())
    }

    /// Raises the `deps_done` count of eligible tickets by 1.
    ///
    /// Returns tickets that are newly `"queued"`.
    async fn raise_deps_done<const M: usize>(
        &self,
        upstream_meta: JobMetadata<M>,
        upstream_job: Job<M>,
        aggregate_dims: &[&'static str],
    ) -> PsqlResult<Vec<Ticket<N>>> {
        let schema = self.client.schema_prefix();
        let stmt = RaiseDepsDoneQuery(schema, self.job_meta, upstream_meta, aggregate_dims);

        let params = upstream_meta
            .dims
            .iter()
            .zip(upstream_job.coordinate)
            .filter(|(d, _)| self.job_meta.dims.contains(d) && !aggregate_dims.contains(d))
            .map(|(_, c)| c);
        let params = SqlParams::from_usize(params)?;

        let rows = self.client.query_stmt(&stmt, &params.borrow()).await?;
        let tickets = rows
            .iter()
            .map(|row| Ticket::from_row(self.job_meta, row))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tickets)
    }

    /// Raises the `deps_quota` count of eligible tickets by resolution's `ub` minus 1.
    ///
    /// Returns tickets that are newly `"queued"`.
    async fn raise_deps_quota<const M: usize>(
        &self,
        upstream_meta: JobMetadata<M>,
        upstream_ticket: Ticket<M>,
        aggregate_dims: &[&'static str],
        ub: usize,
    ) -> PsqlResult<Vec<Ticket<N>>> {
        let schema = self.client.schema_prefix();

        let (cols, values): (Vec<_>, Vec<_>) = upstream_meta
            .dims
            .iter()
            .zip(upstream_ticket.coordinate)
            .filter_map(|(&dim, coord)| {
                if aggregate_dims.contains(&dim) {
                    return None;
                }
                Some((dim, coord.0?))
            })
            .unzip();
        let stmt = RaiseDepsQuotaQuery(schema, self.job_meta, &cols);

        let params = SqlParams::from_usize([ub].into_iter().chain(values))?;

        let rows = self.client.query_stmt(&stmt, &params.borrow()).await?;
        let tickets = rows
            .iter()
            .map(|row| Ticket::from_row(self.job_meta, row))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tickets)
    }

    /// Explodes the ticket along a dimension at a given coordinate.
    ///
    /// Returns tickets affected.
    async fn explode<const M: usize, const IDX: usize>(
        &self,
        res_meta: DimensionMetadata<M>,
        res: Resolution<M>,
    ) -> PsqlResult<Vec<Ticket<N>>> {
        const { assert!(IDX < N) }
        if self.job_meta.dims[IDX] != res_meta.id {
            tracing::warn!("Invalid resolution received for explosion.");
            return Ok(vec![]);
        }

        let schema_prefix = self.client.schema_prefix();
        let pop_stmt = ExplodePopQuery(schema_prefix, self.job_meta, res_meta);

        let params = res_meta
            .deps
            .iter()
            .zip(res.coordinate)
            .filter_map(|(d, c)| self.job_meta.dims.contains(d).then_some(c));
        let params = SqlParams::from_usize(params)?;

        let rows = self.client.query_stmt(&pop_stmt, &params.borrow()).await?;
        let tickets = rows
            .iter()
            .map(|row| Ticket::from_row(self.job_meta, row))
            .collect::<Result<Vec<_>, _>>()?;

        if tickets
            .iter()
            .any(|ticket| ticket.coordinate[IDX].is_some())
        {
            return Err(MetaStorageError::invalid_explosion(
                self.job_meta.id,
                res_meta.id,
            ));
        }

        let new_tickets = tickets
            .iter()
            .flat_map(|ticket| (0..res.ub).map(|x| ticket.with_coordinate::<IDX>(x)))
            .map(|ticket| ticket.update_status())
            .collect::<Vec<_>>();

        let copy_stmt = CopyInQuery(schema_prefix, self.job_meta);
        let sink = self
            .client
            .copy_in::<_, Bytes>(&copy_stmt.to_string())
            .await?;
        let mut sink = Box::pin(sink);
        for ticket in &new_tickets {
            sink.feed(ticket.to_copy_string()?.into())
                .await
                .map_err(PsqlMetaError::from)?;
        }
        sink.close().await.map_err(PsqlMetaError::from)?;

        Ok(tickets)
    }

    /// Marks the ticket corresponding to a given job as done.
    async fn mark_done(&self, job: Job<N>) -> PsqlResult<()> {
        let schema = self.client.schema_prefix();
        let stmt = MarkDoneQuery(schema, self.job_meta);
        let params = SqlParams::from_usize(job.coordinate)?;
        self.client.execute_stmt(&stmt, &params.borrow()).await?;
        Ok(())
    }

    async fn get_status(&self) -> PsqlResult<(i64, i64, i64)> {
        let schema_prefix = self.client.schema_prefix();
        let stmt = GetStatusQuery(schema_prefix);

        let job_id = self.job_meta.id;
        let Some(row) = self.client.query_opt_stmt(&stmt, &[&job_id]).await? else {
            return Err(MetaStorageError::missing_ticket_summary(job_id));
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

/// Helper struct to generate the SQL query dropping a job's ticket summary row.
///
/// Leaves the row absent, exactly as it is for a job whose ticket table has never been
/// initialized, so that the trailing `TicketSummaryInsertQuery` in `init` recreates it.
struct TicketSummaryDeleteQuery<'a, const N: usize>(SchemaPrefix<'a>, JobMetadata<N>);

impl<const N: usize> std::fmt::Display for TicketSummaryDeleteQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;

        write!(
            f,
            "DELETE FROM {schema}ticket_summary WHERE job_id = '{id}';"
        )
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

/// Helper struct to generate the SQL query for inserting tickets for a job.
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

/// Helper struct to generate the SQL query for raising `deps_done` count of tickets.
struct RaiseDepsDoneQuery<'a, const N: usize, const M: usize>(
    SchemaPrefix<'a>,
    JobMetadata<N>,
    JobMetadata<M>,
    &'a [&'static str],
);

impl<const N: usize, const M: usize> std::fmt::Display for RaiseDepsDoneQuery<'_, N, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let self_dims = self.1.dims;
        let received_dims = self.2.dims;
        let aggregate_dims = self.3;

        writeln!(f, "WITH updated AS (")?;
        writeln!(f, "    UPDATE {schema}ticket_{id}")?;
        writeln!(f, "    SET")?;
        writeln!(f, "        deps_done = deps_done + 1,")?;
        writeln!(f, "        status = CASE")?;
        writeln!(f, "            WHEN deps_done + 1 >= deps_quota")?;
        writeln!(f, "                THEN 'queued'::{schema}ticket_status")?;
        writeln!(f, "            ELSE 'waiting'::{schema}ticket_status")?;
        writeln!(f, "        END")?;
        writeln!(f, "    WHERE status = 'waiting'::{schema}ticket_status")?;
        for (idx, dim) in received_dims
            .iter()
            .filter(|d| self_dims.contains(d) && !aggregate_dims.contains(d))
            .enumerate()
        {
            writeln!(f, "        AND {} = ${}", dim, idx + 1)?;
        }
        writeln!(f, "    RETURNING *")?;
        writeln!(f, ")")?;
        writeln!(f, "SELECT *")?;
        writeln!(f, "FROM updated")?;
        write!(f, "WHERE status = 'queued'::{schema}ticket_status;")
    }
}

/// Helper struct to generate the SQL query for raising `deps_done` count of tickets.
struct RaiseDepsQuotaQuery<'a, const N: usize>(SchemaPrefix<'a>, JobMetadata<N>, &'a [&'a str]);

impl<const N: usize> std::fmt::Display for RaiseDepsQuotaQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let cols = self.2;

        writeln!(f, "WITH updated AS (")?;
        writeln!(f, "    UPDATE {schema}ticket_{id}")?;
        writeln!(f, "    SET")?;
        writeln!(f, "        deps_quota = deps_quota + $1 - 1,")?;
        writeln!(f, "        status = CASE")?;
        writeln!(f, "            WHEN deps_done >= deps_quota + $1 - 1")?;
        writeln!(f, "                THEN 'queued'::{schema}ticket_status")?;
        writeln!(f, "            ELSE 'waiting'::{schema}ticket_status")?;
        writeln!(f, "        END")?;
        writeln!(f, "    WHERE status = 'waiting'::{schema}ticket_status")?;
        for (idx, dim) in cols.iter().enumerate() {
            writeln!(f, "        AND {} = ${}", dim, idx + 2)?;
        }
        writeln!(f, "    RETURNING *")?;
        writeln!(f, ")")?;
        writeln!(f, "SELECT *")?;
        writeln!(f, "FROM updated")?;
        write!(f, "WHERE status = 'queued'::{schema}ticket_status;")
    }
}

/// A helper struct to generate the SQL query for popping tickets to be exploded.
struct ExplodePopQuery<'a, const N: usize, const M: usize>(
    SchemaPrefix<'a>,
    JobMetadata<N>,
    DimensionMetadata<M>,
);

impl<const N: usize, const M: usize> std::fmt::Display for ExplodePopQuery<'_, N, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let job_id = self.1.id;

        writeln!(f, "DELETE FROM {schema}ticket_{job_id}")?;
        for (idx, dep) in self
            .2
            .deps
            .iter()
            .filter(|dim| self.1.dims.contains(dim))
            .enumerate()
        {
            if idx == 0 {
                write!(f, "WHERE")?;
            } else {
                write!(f, "    AND")?;
            }
            writeln!(f, " {} = ${}", dep, idx + 1)?;
        }
        write!(f, "RETURNING *;")
    }
}

/// A helper struct to generate the SQL query for copying tickets into the database.
struct CopyInQuery<'a, const N: usize>(SchemaPrefix<'a>, JobMetadata<N>);

impl<const N: usize> std::fmt::Display for CopyInQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = self.0;
        let id = self.1.id;
        let dims = self.1.dims;

        writeln!(f, "COPY {schema}ticket_{id} (")?;
        write!(f, "    ")?;
        for dim in dims {
            write!(f, "{dim}, ")?;
        }
        writeln!(f, "deps_done, deps_quota, status")?;
        writeln!(f, ")")?;
        write!(f, "FROM STDIN WITH (FORMAT csv);")
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
    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};

    use super::*;
    use crate::schema::JobMetadata;

    fn job_alpha() -> JobMetadata<0> {
        JobMetadata {
            id: "alpha",
            spawn_dim: Some("i"),
            dims: [],
            priority: &[],
        }
    }

    fn job_beta() -> JobMetadata<1> {
        JobMetadata {
            id: "beta",
            spawn_dim: Some("j"),
            dims: ["i"],
            priority: &[],
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
        "DELETE FROM test_meta.ticket_summary WHERE job_id = 'beta';"
    )]
    fn test_ticket_summary_delete_query<const N: usize>(
        schema_prefix: SchemaPrefix<'static>,
        #[case] job: JobMetadata<N>,
        #[case] expected: &str,
    ) {
        let stmt = TicketSummaryDeleteQuery(schema_prefix, job).to_string();
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
    #[case::simple(job_beta(), job_alpha(), &[], indoc! { "
        WITH updated AS (
            UPDATE test_meta.ticket_beta
            SET
                deps_done = deps_done + 1,
                status = CASE
                    WHEN deps_done + 1 >= deps_quota
                        THEN 'queued'::test_meta.ticket_status
                    ELSE 'waiting'::test_meta.ticket_status
                END
            WHERE status = 'waiting'::test_meta.ticket_status
            RETURNING *
        )
        SELECT *
        FROM updated
        WHERE status = 'queued'::test_meta.ticket_status;"
    })]
    fn test_raise_deps_done_query<const N: usize, const M: usize>(
        schema_prefix: SchemaPrefix<'static>,
        #[case] job: JobMetadata<N>,
        #[case] upstream_job: JobMetadata<M>,
        #[case] aggregate_dims: &[&'static str],
        #[case] expected: &str,
    ) {
        let stmt = RaiseDepsDoneQuery(schema_prefix, job, upstream_job, aggregate_dims).to_string();
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
