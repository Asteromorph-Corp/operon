use syn::parse_quote;

use crate::{configs::DimensionId, utils::dimension_ident};

/// Generates a type definition for a dimension, which is represented as a `usize`.
///
/// Example:
/// ```rust,ignore
/// pub type IDim = usize;
/// ```
pub(super) fn dimension_definition(dimension_id: &DimensionId) -> syn::ItemType {
    let dim_ident = dimension_ident(dimension_id);

    parse_quote! {
        pub type #dim_ident = usize;
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_dimension_definition() {
        let dimension_id = DimensionId::from("i");
        let item = dimension_definition(&dimension_id);
        let expected: syn::ItemType = parse_quote! {
            pub type IDim = usize;
        };
        assert_eq!(
            item.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
