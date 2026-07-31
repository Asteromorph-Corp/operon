use syn::parse_quote;

use crate::configs::JobConfigMap;
use crate::operon_ident;
use crate::utils::{ticket_enum_ident, to_pascal_case};

/// Generates an enum representing the ticket of any task in the job configuration map.
///
/// # Example
/// ```rust,ignore
/// /// An enum representing the ticket of any task.
/// #[derive(Debug, Clone)]
/// pub enum TicketEnum {
///     Alpha(operon::__private::Ticket<0usize>),
///     Beta(operon::__private::Ticket<1usize>),
///     Gamma(operon::__private::Ticket<1usize>),
///     Delta(operon::__private::Ticket<3usize>),
///     Epsilon(operon::__private::Ticket<2usize>),
///     Zeta(operon::__private::Ticket<1usize>),
/// }
/// ```
pub fn ticket_enum_definition(jobs: &JobConfigMap) -> syn::ItemEnum {
    let operon = operon_ident();
    let ticket_enum_ident = ticket_enum_ident();
    let variants = jobs.values().map(|job| -> syn::Variant {
        // TODO: Fix this
        let variant_ident = to_pascal_case(&job.id);
        let n = job.dims.len();
        parse_quote! {
            #variant_ident(#operon::__private::Ticket<#n>)
        }
    });
    let doc = "An enum representing the ticket of any task.";

    parse_quote! {
        #[doc = #doc]
        #[derive(Debug, Clone)]
        pub enum #ticket_enum_ident {
            #(#variants,)*
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::all_jobs;

    #[rstest]
    fn test_ticket_enum_definition(all_jobs: JobConfigMap) {
        let result = ticket_enum_definition(&all_jobs);
        assert_item_eq(&result, "schema/ticket/ticket_enum_definition.rs");
    }
}
