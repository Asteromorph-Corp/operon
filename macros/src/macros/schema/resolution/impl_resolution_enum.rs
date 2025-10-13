use syn::parse_quote;

use crate::configs::DimensionId;
use crate::utils::{operon_ident, resolution_enum_ident, resolution_ident, variant_ident};

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
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::primary_dim;

    #[rstest]
    fn test_impl_resolution_enum(primary_dim: DimensionId) {
        let result = impl_resolution_enum(&primary_dim);
        assert_item_eq(&result, "schema/resolution/impl_resolution_enum.rs");
    }
}
