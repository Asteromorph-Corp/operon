use crate::meta_storage::{MetaClient, MetaStorageError};
use crate::schema_base::{Job, JobMetadata};
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
    /// Marks the ticket corresponding to a given job as done.
    pub async fn mark_done(&self, job: Job<N>) -> Result<(), MetaStorageError> {
        let schema = self.client.schema_prefix();
        let stmt = MarkDoneQuery(schema, self.job_meta);
        let params = SqlParams::from_usize(job.primary_key)?;
        self.client.execute_stmt(&stmt, &params.borrow()).await?;
        Ok(())
    }
}

/// An helper struct to generate the SQL query for marking a ticket as done for a given job.
struct MarkDoneQuery<'a, const N: usize>(SchemaPrefix<'a>, JobMetadata<N>);

impl<const N: usize> std::fmt::Display for MarkDoneQuery<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let schema = &self.0;
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

#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};

    use super::*;
    use crate::schema_base::JobMetadata;

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
}
