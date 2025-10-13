use syn::parse_quote;

use crate::utils::{
    explode_ident, operon_ident, resolution_ident, ticket_ident, variable_ident, with_ident,
};
use crate::configs::{DimensionConfig, JobConfig};

/// A helper struct to generate the SQL query for popping tickets to be exploded.
struct ExplodePopQuery<'a>(&'a JobConfig, &'a DimensionConfig);

impl std::fmt::Display for ExplodePopQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let job_id = &self.0.id;

        writeln!(f, "DELETE FROM {{schema_prefix}}ticket_{job_id}")?;
        for (i, dep) in self.1.depends_on.iter().enumerate() {
            if i == 0 {
                write!(f, "WHERE")?;
            } else {
                write!(f, "    AND")?;
            }
            writeln!(f, " {} = ${}", dep, i + 1)?;
        }
        write!(f, "RETURNING *;")
    }
}

/// A helper struct to generate the SQL query for copying exploded result into the database.
struct ExplodeCopyInQuery<'a>(&'a JobConfig);

impl std::fmt::Display for ExplodeCopyInQuery<'_> {
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

pub(super) fn fn_explode(job: &JobConfig, dim: &DimensionConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = explode_ident(&job.id, &dim.id);
    let ticket_ident = ticket_ident(&job.id);
    let res_ident = resolution_ident(&dim.id);
    let field_ident = variable_ident(&dim.id);
    let with_fn_name = with_ident(&dim.id);
    let pop_query = ExplodePopQuery(job, dim).to_string();
    let copy_query = ExplodeCopyInQuery(job).to_string();

    let indices = (1..=dim.depends_on.len()).map(syn::Index::from);

    let err_msg = format!(
        "Called `explode({})` on `{}`, but `{}` was resolved",
        dim.id, job.id, dim.id
    );

    parse_quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>,
            resolution: &schema::#res_ident,
        ) -> Result<Vec<schema::#ticket_ident>, #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let pop_stmt = format!(#pop_query);

            let rows = client.query(&pop_stmt, &[#(&i64::try_from(resolution.#indices)?,)*]).await?;
            let tickets = rows
                .iter()
                .map(<schema::#ticket_ident as #operon::schema_base::TicketSql>::from_sql_row)
                .collect::<Result<Vec<_>, _>>()?;

            if tickets.iter().any(|ticket| ticket.#field_ident.is_some()) {
                return Err(#operon::meta_storage::MetaStorageError::InvalidExplosion(
                    #err_msg.into(),
                ));
            }
            let new_tickets = #operon::futures::future::try_join_all(
                tickets
                    .iter()
                    .flat_map(|ticket| (0..resolution.0).map(|ub| ticket.clone().#with_fn_name(ub)))
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
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{dimension_i, dimension_k, job_beta, job_epsilon};

    #[rstest]
    #[case::simple(
        job_beta(),
        dimension_i(),
        indoc! { "
            DELETE FROM {schema_prefix}ticket_beta
            RETURNING *;"
        }
    )]
    #[case::multiple(
        job_epsilon(),
        dimension_k(),
        indoc! {"
            DELETE FROM {schema_prefix}ticket_epsilon
            WHERE i = $1
            RETURNING *;"
        }
    )]
    fn test_explode_pop_query(
        #[case] job: JobConfig,
        #[case] dim: DimensionConfig,
        #[case] expected: &str,
    ) {
        let stmt = ExplodePopQuery(&job, &dim).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(
        job_beta(),
        indoc! { "
            COPY {schema_prefix}ticket_beta (
                i, resolved, deps_count, deps_quota, deps_done, status
            )
            FROM STDIN WITH (FORMAT csv);"
        }
    )]
    fn test_explode_copy_in_query(#[case] job: JobConfig, #[case] expected: &str) {
        let stmt = ExplodeCopyInQuery(&job).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(job_beta(), dimension_i(), "queries/ticket/explode.rs")]
    fn test_explode_fn(
        #[case] job: JobConfig,
        #[case] dim: DimensionConfig,
        #[case] fixture_path: &str,
    ) {
        let result = fn_explode(&job, &dim);
        assert_item_eq(&result, fixture_path);
    }
}
