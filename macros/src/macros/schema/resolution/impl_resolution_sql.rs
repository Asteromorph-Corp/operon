use syn::parse_quote;

use crate::DimensionConfig;
use crate::utils::{
    clear_resolution_ident, get_resolution_ident, init_resolution_ident, operon_ident,
    put_resolution_ident, resolution_ident,
};

/// Generates an implementation of the `ResolutionSql` trait for a given dimension.
///
/// Example:
/// ```rust,ignore
/// #[operon::async_trait::async_trait]
/// #[automatically_derived]
/// impl operon::schema_base::ResolutionSql for IResolution {
///     async fn init_table(
///         client: operon::meta_storage::MetaClient<'_>,
///     ) -> Result<(), operon::meta_storage::MetaStorageError> {
///         queries::init_resolution_i(client).await
///     }
///
///     async fn clear_table(
///         client: operon::meta_storage::MetaClient<'_>,
///     ) -> Result<(), operon::meta_storage::MetaStorageError> {
///         queries::clear_resolution_i(client).await
///     }
///
///     async fn get(
///         client: operon::meta_storage::MetaClient<'_>,
///         primary_key: Self::PrimaryKey,
///     ) -> Result<Option<Self>, operon::meta_storage::MetaStorageError> {
///         queries::get_resolution_i(client).await
///     }
///
///     async fn put(
///         &self,
///         client: operon::meta_storage::MetaClient<'_>,
///     ) -> Result<(), operon::meta_storage::MetaStorageError> {
///         queries::put_resolution_i(client, self).await
///     }
/// }
/// ```
pub(super) fn impl_resolution_sql(dimension: &DimensionConfig) -> syn::ItemImpl {
    let operon = operon_ident();
    let res_ident = resolution_ident(&dimension.id);

    let init_fn_name = init_resolution_ident(&dimension.id);
    let clear_fn_name = clear_resolution_ident(&dimension.id);
    let get_fn_name = get_resolution_ident(&dimension.id);
    let put_fn_name = put_resolution_ident(&dimension.id);

    let indices = (0..dimension.depends_on.len()).map(syn::Index::from);

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl #operon::schema_base::ResolutionSql for #res_ident {
            async fn init_table(
                client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), #operon::meta_storage::MetaStorageError> {
                queries::#init_fn_name(client).await
            }

            async fn clear_table(
                client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), #operon::meta_storage::MetaStorageError> {
                queries::#clear_fn_name(client).await
            }

            async fn get(
                client: #operon::meta_storage::MetaClient<'_>,
                primary_key: Self::PrimaryKey,
            ) -> Result<Option<Self>, #operon::meta_storage::MetaStorageError> {
                queries::#get_fn_name(client, #(primary_key.#indices),*).await
            }

            async fn put(
                &self,
                client: #operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), #operon::meta_storage::MetaStorageError> {
                queries::#put_fn_name(client, self).await
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
    #[case::simple(dimension_i(), "schema/resolution/impl_resolution_sql.rs")]
    fn test_impl_resolution_sql(#[case] dim: DimensionConfig, #[case] fixture_path: &str) {
        let result = impl_resolution_sql(&dim);
        assert_item_eq(&result, fixture_path);
    }
}
