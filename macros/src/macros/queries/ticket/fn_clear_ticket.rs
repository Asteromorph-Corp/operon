use syn::parse_quote;

use crate::{
    configs::JobConfig,
    utils::{clear_ticket_ident, operon_ident},
};

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
    use super::*;

    #[test]
    fn test_clear_ticket_query() {
        let job_beta = JobConfig {
            id: "beta".to_string(),
            from: vec!["a".to_string()],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
        };
        let query = ClearTicketQuery(&job_beta).to_string();
        assert_eq!(query, "TRUNCATE TABLE {schema_prefix}ticket_beta;");
    }

    #[test]
    fn test_fn_clear_ticket() {
        let job_beta = JobConfig {
            id: "beta".to_string(),
            from: vec!["a".to_string()],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
        };
        let tokens = fn_clear_ticket(&job_beta);
        let expected: syn::ItemFn = parse_quote! {
            pub async fn clear_ticket_beta(
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                let schema_prefix = client.schema_prefix();
                let stmt = format!("TRUNCATE TABLE {schema_prefix}ticket_beta;");
                client.execute(&stmt, &[]).await?;
                Ok(())
            }
        };
        assert_eq!(tokens, expected);
    }
}
