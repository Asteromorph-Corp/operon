use syn::parse_quote;

use crate::configs::DimensionId;
use crate::utils::{operon_ident, resolution_ident, resolve_dep_ident, ticket_ident};
use crate::{DimensionConfig, JobConfig};

/// A helper struct to generate the SQL query for popping tickets to be raised.
struct ResolveDepPopQuery<'a>(&'a JobConfig, &'a [&'a DimensionId]);

impl std::fmt::Display for ResolveDepPopQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let job_id = &self.0.id;

        writeln!(f, "DELETE FROM {{schema_prefix}}ticket_{job_id}")?;
        writeln!(f, "WHERE deps_quota IS NULL")?;

        for (i, dim) in self.1.iter().enumerate() {
            writeln!(f, " AND {dim} = ${}", i + 1)?;
        }

        write!(f, "RETURNING *;")
    }
}

/// A helper struct to generate the SQL query for copying exploded result into the database.
struct ResolveDepCopyInQuery<'a>(&'a JobConfig);

impl std::fmt::Display for ResolveDepCopyInQuery<'_> {
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

pub(super) fn fn_resolve_dep(job: &JobConfig, dim: &DimensionConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = resolve_dep_ident(&job.id, &dim.id);
    let ticket_ident = ticket_ident(&job.id);
    let res_ident = resolution_ident(&dim.id);

    let params = dim
        .depends_on
        .iter()
        .filter(|d| job.dims.contains(d))
        .collect::<Vec<_>>();
    let param_vars = params
        .iter()
        .map(|d| syn::Index::from(dim.depends_on.iter().position(|x| x == *d).unwrap() + 1));

    let pop_query = ResolveDepPopQuery(job, &params).to_string();
    let copy_query = ResolveDepCopyInQuery(job).to_string();

    parse_quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>,
            resolution: &schema::#res_ident,
        ) -> Result<Vec<schema::#ticket_ident>, #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let params = [
                #(resolution.#param_vars),*
            ]
            .into_iter()
            .map(|v| i64::try_from(v))
            .collect::<Result<Vec<_>, _>>()?;

            let pop_stmt = format!(#pop_query);
            let rows = client.query(
                &pop_stmt,
                &params.iter().map(|p| p as &(dyn #operon::postgres_types::ToSql + Sync)).collect::<Vec<_>>()
            ).await?;

            let tickets = rows
                .iter()
                .map(<schema::#ticket_ident as #operon::schema_base::TicketSql>::from_sql_row)
                .collect::<Result<Vec<_>, _>>()?;
            let new_tickets = #operon::futures::future::try_join_all(
                tickets
                    .into_iter()
                    .map(|ticket| #operon::schema_base::Ticket::resolve_dependency_quota(ticket, client))
            )
            .await?;

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
    use crate::test_utils::simple_pipeline::{dimension_i, job_beta};

    #[rstest]
    #[case::simple(job_beta(), dimension_i())]
    fn test_fn_resolve_dep(#[case] job: JobConfig, #[case] dim: DimensionConfig) {
        let result = fn_resolve_dep(&job, &dim);
        assert_item_eq(&result, "queries/ticket/resolve_dep.rs");
    }
}
