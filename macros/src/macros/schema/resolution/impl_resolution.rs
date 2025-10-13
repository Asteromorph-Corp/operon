use syn::parse_quote;

use crate::DimensionConfig;
use crate::utils::{dimension_ident, operon_ident, resolution_ident};

/// Generates an implementation of the `Resolution` trait for a given dimension.
///
/// Example:
///
/// ```rust,ignore
/// #[automatically_derived]
/// impl operon::schema_base::Resolution for JResolution {
///     type PrimaryKey = (IDim,);
///
///     #[allow(clippy::unused_unit)]
///     fn primary_key(&self) -> Self::PrimaryKey {
///         (self.1,)
///     }
///
///     fn ub(&self) -> usize {
///         self.0
///     }
///
///     #[allow(unused_variables)]
///     fn new(ub: usize, primary_key: Self::PrimaryKey) -> Self {
///         Self(ub, primary_key.0,)
///     }
/// }
/// ```
pub(super) fn impl_resolution(dimension: &DimensionConfig) -> syn::ItemImpl {
    let operon = operon_ident();
    let res_ident = resolution_ident(&dimension.id);
    let dep_dim_idents = dimension.depends_on.iter().map(dimension_ident);

    let indices = (1..=dep_dim_idents.len()).map(syn::Index::from);
    let pkey_indices = (0..dep_dim_idents.len()).map(syn::Index::from);

    parse_quote! {
        #[automatically_derived]
        impl #operon::schema_base::Resolution for #res_ident {
            type PrimaryKey = (#(#dep_dim_idents,)*);

            #[allow(clippy::unused_unit)]
            fn primary_key(&self) -> Self::PrimaryKey {
                (#(self.#indices,)*)
            }

            fn ub(&self) -> usize {
                self.0
            }

            #[allow(unused_variables)]
            fn new(ub: usize, primary_key: Self::PrimaryKey) -> Self {
                Self(
                    ub,
                    #(primary_key.#pkey_indices,)*
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{dimension_i, dimension_j};

    #[rstest]
    #[case::simple(dimension_i(), "schema/resolution/impl_resolution.simple.rs")]
    #[case::with_dependency(dimension_j(), "schema/resolution/impl_resolution.with_dependency.rs")]
    fn test_impl_resolution(#[case] dim: DimensionConfig, #[case] fixture_path: &str) {
        let result = impl_resolution(&dim);
        assert_item_eq(&result, fixture_path);
    }
}
