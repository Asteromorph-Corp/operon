use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{dimension_ident, operon_ident, ticket_ident, variable_ident};

/// Generates a struct definition for a ticket for a given job.
///
/// Example:
/// ```rust,ignore
/// #[derive(Debug, Clone, Default)]
/// pub struct BetaTicket {
///     pub i: operon::schema_base::TicketDepCount<IDim>,
///     deps_count: usize,
///     deps_quota: usize,
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
            deps_quota: usize,
            // TODO: remove this field.
            deps_done: bool,
            pub status: #operon::schema_base::TicketStatus,
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
    #[case(job_beta(), "schema/ticket/ticket_definition.rs")]
    fn test_ticket_definition(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let result = ticket_definition(&job);
        assert_item_eq(&result, fixture_path);
    }
}
