use syn::parse_quote;

use crate::configs::{DimensionConfig, JobConfig};
use crate::utils::{operon_ident, raise_quota_ident, resolution_ident, ticket_ident};

/// A helper struct to generate the SQL query for popping tickets to be raised.
struct RaiseQuotaPopQuery<'a>(&'a JobConfig, &'a DimensionConfig);

impl std::fmt::Display for RaiseQuotaPopQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let job_id = &self.0.id;

        writeln!(f, "DELETE FROM {{schema_prefix}}ticket_{job_id}")?;
        for (idx, dim) in self
            .1
            .depends_on
            .iter()
            .filter(|dim| self.0.dims.contains(dim))
            .enumerate()
        {
            if idx == 0 {
                write!(f, "WHERE {dim} = ${}", idx + 1)?;
            } else {
                write!(f, " AND {dim} = ${}", idx + 1)?;
            }
        }
        writeln!(f)?;
        write!(f, "RETURNING *;")
    }
}

/// A helper struct to generate the SQL query for copying exploded result into the database.
struct RaiseQuotaCopyInQuery<'a>(&'a JobConfig);

impl std::fmt::Display for RaiseQuotaCopyInQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let job_id = &self.0.id;

        writeln!(f, "COPY {{schema_prefix}}ticket_{job_id} (")?;
        write!(f, "    ")?;
        for dim in &self.0.dims {
            write!(f, "{dim}, ")?;
        }
        writeln!(f, "resolved, deps_count, deps_quota, deps_done, status")?;
        writeln!(f, ")")?;
        write!(f, "FROM STDIN WITH (FORMAT csv);")
    }
}

pub(super) fn fn_raise_quota(job: &JobConfig, dim: &DimensionConfig) -> syn::ItemFn {
    // TODO: only define raise_dep for a valid combination of jobs.
    let operon = operon_ident();
    let fn_name = raise_quota_ident(&job.id, &dim.id);
    let ticket_ident = ticket_ident(&job.id);
    let res_ident = resolution_ident(&dim.id);
    let pop_query = RaiseQuotaPopQuery(job, dim).to_string();
    let copy_query = RaiseQuotaCopyInQuery(job).to_string();

    let indices = dim.depends_on.iter().enumerate().filter_map(|(idx, dim)| {
        if job.dims.contains(dim) {
            Some(syn::Index::from(idx + 1))
        } else {
            None
        }
    });

    parse_quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>,
            res: &schema::#res_ident,
        ) -> Result<Vec<schema::#ticket_ident>, #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let pop_stmt = format!(#pop_query);

            let rows = client.query(&pop_stmt, &[#(&i64::try_from(res.#indices)?,)*]).await?;
            let tickets = rows
                .iter()
                .map(<schema::#ticket_ident as #operon::schema_base::TicketSql>::from_sql_row)
                .collect::<Result<Vec<_>, _>>()?;
            let new_tickets = tickets
                .into_iter()
                .map(|ticket| #operon::schema_base::Ticket::raise_dependency_quota(ticket, res.0))
                .collect::<Vec<_>>();

            let copy_stmt = format!(#copy_query);
            let sink = client.copy_in::<_, #operon::bytes::Bytes>(&copy_stmt).await?;
            let mut sink = Box::pin(sink);
            for ticket in &new_tickets {
                #operon::futures::SinkExt::feed(
                    &mut sink,
                    #operon::schema_base::TicketSql::to_sql_copy_params(ticket)?.into(),
                )
                .await?;
            }
            #operon::futures::SinkExt::close(&mut sink).await?;

            let ready_tickets = new_tickets
                .into_iter()
                .filter(#operon::schema_base::Ticket::is_ready)
                .collect::<Vec<_>>();
            Ok(ready_tickets)
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{dimension_j, job_epsilon};

    #[rstest]
    #[case::simple(job_epsilon(), dimension_j(), "queries/ticket/raise_quota.rs")]
    fn test_fn_raise_quota(
        #[case] job: JobConfig,
        #[case] dim: DimensionConfig,
        #[case] fixture_path: &str,
    ) {
        let item = fn_raise_quota(&job, &dim);
        assert_item_eq(&item, fixture_path);
    }
}
