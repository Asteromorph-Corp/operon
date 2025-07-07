use syn::parse_quote;

use crate::{
    configs::DimensionConfigMap,
    utils::{resolution_enum_ident, resolution_ident, variant_ident},
};

/// Generates an enum representing the resolution of any dimension.
///
/// Example:
/// ```rust,ignore
/// #[doc = "An enum representing the resolution of any dimension."]
/// #[derive(Debug, Clone)]
/// pub enum ResolutionEnum {
///     I(IResolution),
///     J(JResolution),
/// }
/// ```
pub(super) fn resolution_enum(dimensions: &DimensionConfigMap) -> syn::ItemEnum {
    let res_enum_ident = resolution_enum_ident();
    let variants = dimensions.keys().map(|dim| -> syn::Variant {
        let variant_ident = variant_ident(dim);
        let inner = resolution_ident(dim);
        parse_quote! {
            #variant_ident(#inner)
        }
    });

    let doc = "An enum representing the resolution of any dimension.";

    syn::parse_quote! {
        #[doc = #doc]
        #[derive(Debug, Clone)]
        pub enum #res_enum_ident {
            #(#variants,)*
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::DimensionConfig;

    use super::*;

    #[test]
    fn test_resolution_enum() {
        let dimensions = DimensionConfigMap::from_iter([
            (
                "i".to_string(),
                DimensionConfig {
                    id: "i".to_string(),
                    depends_on: vec![],
                },
            ),
            (
                "j".to_string(),
                DimensionConfig {
                    id: "j".to_string(),
                    depends_on: vec!["i".to_string()],
                },
            ),
        ]);

        let result = resolution_enum(&dimensions);
        let expected: syn::ItemEnum = parse_quote! {
            #[doc = "An enum representing the resolution of any dimension."]
            #[derive(Debug, Clone)]
            pub enum ResolutionEnum {
                I(IResolution),
                J(JResolution),
            }
        };

        assert_eq!(result, expected);
    }
}
