use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{clear_ticket_ident, operon_ident};

/// Helper struct to generate the SQL query for clearing a ticket table.
struct ClearTicketQuery<'a>(&'a JobConfig);

impl std::fmt::Display for ClearTicketQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TRUNCATE TABLE {{schema_prefix}}ticket_{};", self.0.id)
    }
}

/// Generates the `clear_ticket_*` function for a given job.
///
/// Example:
/// ```rust,ignore
/// pub async fn clear_ticket_beta(
///     client: operon::meta_storage::MetaClient<'_>,
/// ) -> Result<(), operon::meta_storage::MetaStorageError> {
///     let schema_prefix = client.schema_prefix();
///     let stmt = format!("TRUNCATE TABLE {schema_prefix}ticket_beta;");
///     client.execute(&stmt, &[]).await?;
///     Ok(())
/// }
/// ```
pub(super) fn fn_clear_ticket(job: &JobConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = clear_ticket_ident(&job.id);
    let stmt = ClearTicketQuery(job).to_string();

    parse_quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>,
        ) -> Result<(), #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(#stmt);
            client.execute(&stmt, &[]).await?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::job_beta;

    #[rstest]
    #[case::simple(job_beta(), "TRUNCATE TABLE {schema_prefix}ticket_beta;")]
    fn test_clear_ticket_query(#[case] job: JobConfig, #[case] expected: &str) {
        let stmt = ClearTicketQuery(&job).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(job_beta(), "queries/ticket/clear_ticket.rs")]
    fn test_fn_clear_ticket(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let result = fn_clear_ticket(&job);
        assert_item_eq(&result, fixture_path);
    }
}
