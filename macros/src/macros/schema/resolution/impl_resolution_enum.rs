use syn::parse_quote;

use crate::{
    configs::DimensionId,
    utils::{operon_ident, resolution_enum_ident, resolution_ident, variant_ident},
};

/// Generates an implementation of the `ResolutionEnum` trait for a given resolution enum.
///
/// Example:
/// ```rust, ignore
/// #[automatically_derived]
/// impl operon::schema_base::ResolutionEnum for ResolutionEnum {
///     fn primary(resolution: usize) -> Self {
///         Self::I(IResolution(resolution))
///     }
/// }
/// ```
pub(super) fn impl_resolution_enum(primary_dimension: &DimensionId) -> syn::ItemImpl {
    let operon = operon_ident();
    let res_enum_ident = resolution_enum_ident();
    let res_ident = resolution_ident(primary_dimension);
    let primary_variant_ident = variant_ident(primary_dimension);

    parse_quote! {
        impl #operon::schema_base::ResolutionEnum for #res_enum_ident {
            fn primary(resolution: usize) -> Self {
                Self::#primary_variant_ident(#res_ident(resolution))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::*;

    #[test]
    fn test_impl_resolution_enum() {
        let primary_dimension = DimensionId::from("i");
        let result = impl_resolution_enum(&primary_dimension);
        let expected: syn::ItemImpl = parse_quote! {
            impl operon::schema_base::ResolutionEnum for ResolutionEnum {
                fn primary(resolution: usize) -> Self {
                    Self::I(IResolution(resolution))
                }
            }
        };
        assert_eq!(
            result.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
