use syn::parse_quote;

use crate::configs::TaskConfigMap;
use crate::operon_ident;
use crate::utils::{ticket_enum_ident, to_pascal_case};

/// Generates an enum representing a ticket for any task in the pipeline.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/schema/ticket/ticket_enum_definition.rs"))]
/// ```
pub(crate) fn ticket_enum_definition(tasks: &TaskConfigMap) -> syn::ItemEnum {
    let operon = operon_ident();
    let ticket_enum_ident = ticket_enum_ident();
    let variants = tasks.values().map(|task| -> syn::Variant {
        let variant_ident = to_pascal_case(&task.id);
        let n = task.dims.len();
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
    use crate::test_utils::simple_pipeline::all_tasks;

    #[rstest]
    fn test_ticket_enum_definition(all_tasks: TaskConfigMap) {
        let result = ticket_enum_definition(&all_tasks);
        assert_item_eq(&result, "schema/ticket/ticket_enum_definition.rs");
    }
}
