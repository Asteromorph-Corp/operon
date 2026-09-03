use syn::parse_quote;

use crate::utils::{operon_ident, ticket_enum_ident};

/// Generates an implementation of the `JobEnum` trait for the `JobEnum` type.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/schema/ticket/impl_ticket_enum.rs") )]
/// ```
pub fn impl_ticket_enum() -> syn::ItemImpl {
    let operon = operon_ident();
    let ticket_enum_ident = ticket_enum_ident();

    parse_quote! {
        #[automatically_derived]
        impl #operon::__private::TicketEnum for #ticket_enum_ident {}
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;

    #[rstest]
    fn test_impl_ticket_enum() {
        let result = impl_ticket_enum();
        assert_item_eq(&result, "schema/ticket/impl_ticket_enum.rs");
    }
}
