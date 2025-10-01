use syn::parse_quote;

use crate::JobConfig;
use crate::utils::{dimension_ident, operon_ident, ticket_ident, variable_ident};

/// Generates a struct definition for a ticket for a given job.
///
/// Example:
/// ```rust,ignore
/// #[derive(Debug, Clone, Default)]
/// pub struct BetaTicket {
///     pub i: operon::schema_base::TicketDepCount<IDim>,
///     deps_count: usize,
///     deps_quota: Option<usize>,
///     deps_done: bool,
///     pub status: operon::schema_base::TicketStatus,
/// }
/// ```
pub(super) fn ticket_definition(job: &JobConfig) -> syn::ItemStruct {
    let operon = operon_ident();
    let ticket_ident = ticket_ident(&job.id);
    let dim_fields = job.dims.iter().map(|dim| -> syn::Field {
        let field_ident = variable_ident(dim);
        let dim_ident = dimension_ident(dim);

        parse_quote!(
            pub #field_ident: operon::schema_base::TicketDepCount<#dim_ident>
        )
    });

    parse_quote! {
        #[derive(Debug, Clone, Default)]
        pub struct #ticket_ident {
            #(#dim_fields,)*
            deps_count: usize,
            deps_quota: Option<usize>,
            // TODO: remove this field.
            deps_done: bool,
            pub status: #operon::schema_base::TicketStatus,
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;
    use crate::configs::JobArg;

    #[test]
    fn test_ticket_definition() {
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
        let item = ticket_definition(&job);
        let expected: syn::ItemStruct = parse_quote! {
            #[derive(Debug, Clone, Default)]
            pub struct BetaTicket {
                pub i: operon::schema_base::TicketDepCount<IDim>,
                deps_count: usize,
                deps_quota: Option<usize>,
                deps_done: bool,
                pub status: operon::schema_base::TicketStatus,
            }
        };
        assert_eq!(item, expected);
    }
}
