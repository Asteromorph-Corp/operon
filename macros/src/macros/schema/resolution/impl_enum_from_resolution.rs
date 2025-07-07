use syn::parse_quote;

use crate::{
    DimensionConfig,
    utils::{resolution_enum_ident, resolution_ident, variant_ident},
};

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
    use quote::ToTokens;

    use super::*;

    #[test]
    fn test_impl_enum_from_resolution() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let result_i = impl_enum_from_resolution(&dimension_i);
        let expected_i: syn::ItemImpl = parse_quote! {
            #[automatically_derived]
            impl From<IResolution> for ResolutionEnum {
                fn from(resolution: IResolution) -> Self {
                    Self::I(resolution)
                }
            }
        };

        assert_eq!(
            result_i.to_token_stream().to_string(),
            expected_i.to_token_stream().to_string()
        );
    }
}
