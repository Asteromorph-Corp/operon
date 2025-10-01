use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{get_all_ident, operon_ident, ticket_ident};

/// Helper struct to generate the SQL query for getting all tickets for a given job.
struct GetAllTicketQuery<'a>(&'a JobConfig);

impl std::fmt::Display for GetAllTicketQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SELECT * FROM {{schema_prefix}}ticket_{} WHERE status = $1;",
            self.0.id
        )
    }
}

/// Generates the `get_all_*` function for a given job.
///
/// Example:
/// ```rust,ignore
/// pub async fn get_all_beta(
///     client: operon::meta_storage::MetaClient<'_>,
///     status: operon::schema_base::TicketStatus,
/// ) -> Result<Vec<schema::BetaTicket>, operon::meta_storage::MetaStorageError> {
///     let schema_prefix = client.schema_prefix();
///     let stmt = format!("SELECT * FROM {schema_prefix}ticket_beta WHERE status = $1;");
///     let rows = client.query(&stmt, &[&status]).await?;
///     let jobs = rows
///         .iter()
///         .map(|row| <schema::BetaTicket as operon::schema_base::TicketSql>::from_sql_row(row))
///         .collect::<Result<Vec<_>, _>>()?;
///     Ok(jobs)
/// }
/// ```
pub(super) fn fn_get_all(job: &JobConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = get_all_ident(&job.id);
    let ticket_ident = ticket_ident(&job.id);
    let stmt = GetAllTicketQuery(job).to_string();

    parse_quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>,
            status: #operon::schema_base::TicketStatus,
        ) -> Result<Vec<schema::#ticket_ident>, #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(#stmt);
            let rows = client.query(&stmt, &[&status]).await?;
            let jobs = rows.
                iter()
                .map(|row| <schema::#ticket_ident as #operon::schema_base::TicketSql>::from_sql_row(row))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(jobs)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configs::JobArg;

    #[test]
    fn test_get_all_ticket_query() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec![JobArg {
                id: "a".to_string(),
                over: vec![],
            }],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
            pool_size: 8,
        };
        let query = GetAllTicketQuery(&job).to_string();
        let expected = "SELECT * FROM {schema_prefix}ticket_beta WHERE status = $1;";
        assert_eq!(query, expected);
    }

    #[test]
    fn test_fn_get_all() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec![JobArg {
                id: "a".to_string(),
                over: vec![],
            }],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
            pool_size: 8,
        };
        let tokens = fn_get_all(&job);
        let expected: syn::ItemFn = parse_quote! {
            pub async fn get_all_beta(
                client: operon::meta_storage::MetaClient<'_>,
                status: operon::schema_base::TicketStatus,
            ) -> Result<Vec<schema::BetaTicket>, operon::meta_storage::MetaStorageError> {
                let schema_prefix = client.schema_prefix();
                let stmt = format!("SELECT * FROM {schema_prefix}ticket_beta WHERE status = $1;");
                let rows = client.query(&stmt, &[&status]).await?;
                let jobs = rows
                    .iter()
                    .map(|row| <schema::BetaTicket as operon::schema_base::TicketSql>::from_sql_row(row))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(jobs)
            }
        };

        assert_eq!(tokens, expected);
    }
}
