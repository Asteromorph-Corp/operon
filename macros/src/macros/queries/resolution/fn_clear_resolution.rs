use syn::parse_quote;

use crate::configs::DimensionConfig;
use crate::utils::{clear_resolution_ident, operon_ident};

/// Helper struct to generate the SQL query for clearing a dimension's resolution.
struct ClearResolutionQuery<'a>(&'a DimensionConfig);

impl std::fmt::Display for ClearResolutionQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TRUNCATE TABLE {{schema_prefix}}dimension_{};",
            self.0.id
        )
    }
}

/// Generates the `clear_resolution_*` function for a given dimension.
///
/// Example:
/// ```rust,ignore
/// pub async fn clear_resolution_i(
///   client: operon::meta_storage::MetaClient<'_>,
/// ) -> Result<(), operon::meta_storage::MetaStorageError> {
///     let schema_prefix = client.schema_prefix();
///     
///     let stmt = format!("TRUNCATE TABLE {schema_prefix}dimension_i;");
///     client.execute(&stmt, &[]).await?;
///     Ok(())
/// }
/// ```
pub(super) fn fn_clear_resolution(dimension: &DimensionConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = clear_resolution_ident(&dimension.id);
    let stmt = ClearResolutionQuery(dimension).to_string();

    parse_quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>,
        ) -> Result<(), #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(#stmt);
            client.execute(&stmt, &[]).await?;
            Ok(())
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
    #[case::simple(dimension_i(), "TRUNCATE TABLE {schema_prefix}dimension_i;")]
    fn test_clear_resolution_query(#[case] dim: DimensionConfig, #[case] expected: &str) {
        let stmt = ClearResolutionQuery(&dim).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(dimension_i(), "queries/resolution/clear_resolution.rs")]
    fn test_fn_clear_resolution(#[case] dim: DimensionConfig, #[case] fixture_path: &str) {
        let result = fn_clear_resolution(&dim);
        assert_item_eq(&result, fixture_path);
    }
}
