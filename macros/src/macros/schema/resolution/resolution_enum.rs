use syn::parse_quote;

use crate::configs::DimensionConfigMap;
use crate::utils::{resolution_enum_ident, resolution_ident, variant_ident};

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
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::all_dimensions;

    #[rstest]
    fn test_resolution_enum(all_dimensions: DimensionConfigMap) {
        let result = resolution_enum(&all_dimensions);
        assert_item_eq(&result, "schema/resolution/resolution_enum.rs");
    }
}
