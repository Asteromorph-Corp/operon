use syn::parse_quote;

use crate::configs::DimensionConfigMap;
use crate::operon_ident;
use crate::utils::{resolution_enum_ident, to_pascal_case};

/// Generates an enum representing the resolution of any dimension.
///
/// # Example
/// ```rust,ignore
/// #[derive(Debug, Clone)]
/// pub enum ResolutionEnum {
///     I(operon::__private::Resolution<0usize>),
///     J(operon::__private::Resolution<1usize>),
///     K(operon::__private::Resolution<1usize>),
/// }
/// ```
pub fn resolution_enum_definition(dimensions: &DimensionConfigMap) -> syn::ItemEnum {
    let operon = operon_ident();
    let res_enum_ident = resolution_enum_ident();
    let variants = dimensions.values().map(|dim| -> syn::Variant {
        let variant_ident = to_pascal_case(&dim.id);
        let n = dim.depends_on.len();
        parse_quote! {
            #variant_ident(#operon::__private::Resolution<#n>)
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
        let result = resolution_enum_definition(&all_dimensions);
        assert_item_eq(&result, "schema/resolution/resolution_enum_definition.rs");
    }
}
