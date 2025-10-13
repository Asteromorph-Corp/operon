use syn::parse_quote;

/// Generates the `PrimarySpec` struct definition for the primary entity and dimension.
///
/// Example:
/// ```rust,ignore
/// #[derive(Debug, Clone, Copy)]
/// pub struct PrimarySpec;
/// ```
pub fn primary_spec_definition() -> syn::ItemStruct {
    parse_quote! {
        #[derive(Debug, Clone, Copy)]
        pub struct PrimarySpec;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::assert_item_eq;

    #[test]
    fn test_primary_spec_definition() {
        let item = primary_spec_definition();
        assert_item_eq(&item, "spec/primary_spec_definition.rs");
    }
}
