use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{operon_ident, put_ticket_ident, ticket_ident};

/// Helper struct to generate the SQL query for inserting default tickets for a job.
struct PutTicketQuery<'a>(&'a JobConfig);

impl std::fmt::Display for PutTicketQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "INSERT INTO {{schema_prefix}}ticket_{} (", self.0.id)?;
        for dim in &self.0.dims {
            write!(f, "{dim}, ")?;
        }
        writeln!(f, "resolved, deps_count, deps_quota, deps_done, status)")?;

        write!(f, "VALUES (")?;
        for i in 1..=(self.0.dims.len() + 5) {
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

/// Generates the `clear_ticket_*` function for a given job.
///
/// Example:
/// ```rust,ignore
/// pub async fn put_ticket_beta(
///     client: operon::meta_storage::MetaClient<'_>,
///     ticket: &schema::BetaTicket,
/// ) -> Result<(), operon::meta_storage::MetaStorageError> {
///     let schema_prefix = client.schema_prefix();
///     let stmt = format!(#stmt);
///     let params = operon::schema_base::TicketSql::to_sql_insert_params(ticket)?;
///     let params = params
///         .iter()
///         .map(|p| p.as_ref() as &(dyn operon::postgres_typesToSql + Sync))
///         .collect::<Vec<_>>();
///
///     client.execute(&stmt, &params).await?;
///     Ok(())
/// }
/// ```
pub(super) fn fn_put_ticket(job: &JobConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let ticket_ident = ticket_ident(&job.id);
    let fn_name = put_ticket_ident(&job.id);
    let stmt = PutTicketQuery(job).to_string();

    parse_quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>,
            ticket: &schema::#ticket_ident,
        ) -> Result<(), #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(#stmt);
            let params = operon::schema_base::TicketSql::to_sql_insert_params(ticket)?;
            let params = params
                .iter()
                .map(|p| p.as_ref() as &(dyn operon::postgres_types::ToSql + Sync)).collect::<Vec<_>>();

            client.execute(&stmt, &params).await?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{job_alpha, job_beta};

    #[rstest]
    #[case::empty(
        job_alpha(),
        indoc! {"
            INSERT INTO {schema_prefix}ticket_alpha (resolved, deps_count, deps_quota, deps_done, status)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT DO NOTHING;"
        }
    )]
    #[case::simple(
        job_beta(),
        indoc! {"
            INSERT INTO {schema_prefix}ticket_beta (i, resolved, deps_count, deps_quota, deps_done, status)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT DO NOTHING;"
        }
    )]
    fn test_put_ticket_query(#[case] job: JobConfig, #[case] expected: &str) {
        let stmt = PutTicketQuery(&job).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(job_beta(), "queries/ticket/put_ticket.rs")]
    fn test_fn_put_ticket(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let result = fn_put_ticket(&job);
        assert_item_eq(&result, fixture_path);
    }
}
