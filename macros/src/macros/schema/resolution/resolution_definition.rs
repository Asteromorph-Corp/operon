use syn::parse_quote;

use crate::{
    configs::DimensionConfig,
    utils::{dimension_ident, resolution_ident},
};

/// Generates a struct definition for the resolution of a dimension.
///
/// Example:
/// ```rust,ignore
/// #[doc = "A struct representing the resolution of dimension i"]
/// #[derive(Debug, Clone, Copy)]
/// pub struct IResolution(pub IDim, pub JDim, pub KDim);
/// ```
pub(super) fn resolution_definition(dimension: &DimensionConfig) -> syn::ItemStruct {
    let res_ident = resolution_ident(&dimension.id);
    let dim_ident = dimension_ident(&dimension.id);
    let dep_dim_idents = dimension.depends_on.iter().map(dimension_ident);

    let doc = format!(
        "A struct representing the resolution of dimension {}",
        dimension.id
    );

    parse_quote! {
        #[doc = #doc]
        #[derive(Debug, Clone, Copy)]
        pub struct #res_ident(pub #dim_ident, #(pub #dep_dim_idents),*);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;

    #[test]
    fn test_resolution_definition() {
        let dimension = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec!["j".to_string(), "k".to_string()],
        };
        let result = resolution_definition(&dimension);
        let expected: syn::ItemStruct = parse_quote! {
            #[doc = "A struct representing the resolution of dimension i"]
            #[derive(Debug, Clone, Copy)]
            pub struct IResolution(pub IDim, pub JDim, pub KDim);
        };
        assert_eq!(
            result.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
