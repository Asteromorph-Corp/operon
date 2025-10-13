use syn::parse_quote;

use crate::configs::DimensionConfig;
use crate::utils::{resolution_enum_ident, resolution_ident, variant_ident};

/// Generates an implementation of `From` trait for converting a resolution type
///
/// Example:
/// ```rust,ignore
/// #[automatically_derived]
/// impl From<IResolution> for ResolutionEnum {
///     fn from(resolution: IResolution) -> Self {
///         Self::I(resolution)
///     }
/// }
/// ```
pub(super) fn impl_enum_from_resolution(dimension: &DimensionConfig) -> syn::ItemImpl {
    let res_ident = resolution_ident(&dimension.id);
    let res_enum_ident = resolution_enum_ident();
    let variant_ident = variant_ident(&dimension.id);

    parse_quote! {
        #[automatically_derived]
        impl From<#res_ident> for #res_enum_ident {
            fn from(resolution: #res_ident) -> Self {
                Self::#variant_ident(resolution)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::dimension_i;

    #[rstest]
    #[case::simple(dimension_i(), "schema/resolution/impl_enum_from_resolution.rs")]
    fn test_impl_enum_from_resolution(#[case] dim: DimensionConfig, #[case] fixture_path: &str) {
        let result = impl_enum_from_resolution(&dim);
        assert_item_eq(&result, fixture_path);
    }
}
