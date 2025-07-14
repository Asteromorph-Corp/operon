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

    #[test]
    fn test_primary_spec_definition() {
        let expected: syn::ItemStruct = parse_quote! {
            #[derive(Debug, Clone, Copy)]
            pub struct PrimarySpec;
        };
        assert_eq!(primary_spec_definition(), expected);
    }
}
