use syn::parse_quote;

use crate::utils::{operon_ident, resolution_enum_ident};

/// Generates an implementation of the `ResolutionEnum` trait for a given resolution enum.
///
/// # Example
/// ```rust,ignore
/// #[automatically_derived]
/// impl operon::schema::ResolutionEnum for ResolutionEnum {}
/// ```
pub fn impl_resolution_enum() -> syn::ItemImpl {
    let operon = operon_ident();
    let res_enum_ident = resolution_enum_ident();

    parse_quote! {
        #[automatically_derived]
        impl #operon::schema::ResolutionEnum for #res_enum_ident {}
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;

    #[rstest]
    fn test_impl_resolution_enum() {
        let result = impl_resolution_enum();
        assert_item_eq(&result, "schema/resolution/impl_resolution_enum.rs");
    }
}
