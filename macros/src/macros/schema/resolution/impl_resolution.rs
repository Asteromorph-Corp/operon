use syn::parse_quote;

use crate::{
    DimensionConfig,
    utils::{dimension_ident, operon_ident, resolution_ident},
};

/// Generates an implementation of the `Resolution` trait for a given dimension.
///
/// Example:
///
/// ```rust,ignore
/// #[automatically_derived]
/// impl operon::schema_base::Resolution for JResolution {
///     type PrimaryKey = (I,);
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
    use quote::ToTokens;

    use super::*;

    #[test]
    fn test_impl_resolution() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let result_i = impl_resolution(&dimension_i);
        let expected_i: syn::ItemImpl = parse_quote! {
            #[automatically_derived]
            impl operon::schema_base::Resolution for IResolution {
                type PrimaryKey = ();

                #[allow(clippy::unused_unit)]
                fn primary_key(&self) -> Self::PrimaryKey {
                    ()
                }

                fn ub(&self) -> usize {
                    self.0
                }

                #[allow(unused_variables)]
                fn new(ub: usize, primary_key: Self::PrimaryKey) -> Self {
                    Self(ub,)
                }
            }
        };

        assert_eq!(
            result_i.to_token_stream().to_string(),
            expected_i.to_token_stream().to_string()
        );

        let dimension_k = DimensionConfig {
            id: "j".to_string(),
            depends_on: vec!["i".to_string()],
        };
        let result = impl_resolution(&dimension_k);
        let expected: syn::ItemImpl = parse_quote! {
            #[automatically_derived]
            impl operon::schema_base::Resolution for JResolution {
                type PrimaryKey = (I,);

                #[allow(clippy::unused_unit)]
                fn primary_key(&self) -> Self::PrimaryKey {
                    (self.1,)
                }

                fn ub(&self) -> usize {
                    self.0
                }

                #[allow(unused_variables)]
                fn new(ub: usize, primary_key: Self::PrimaryKey) -> Self {
                    Self(ub, primary_key.0,)
                }
            }
        };
        assert_eq!(
            result.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
