use syn::parse_quote;

use crate::{
    configs::DimensionConfig,
    utils::{clear_resolution_ident, operon_ident},
};

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
    use super::*;

    #[test]
    fn test_clear_resolution_query() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let clear_resolution = ClearResolutionQuery(&dimension_i);

        let stmt_i = "TRUNCATE TABLE {schema_prefix}dimension_i;";

        assert_eq!(clear_resolution.to_string(), stmt_i);
    }

    #[test]
    fn test_fn_clear_resolution() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let result_i = fn_clear_resolution(&dimension_i);

        let stmt_i = "TRUNCATE TABLE {schema_prefix}dimension_i;"; // Note: placing this string literal inside the quote! macro results in a `\n` instead of `\\n`, causing the test to fail.
        let expected_i: syn::ItemFn = parse_quote! {
            pub async fn clear_resolution_i(
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                let schema_prefix = client.schema_prefix();
                let stmt = format!(#stmt_i);
                client.execute(&stmt, &[]).await?;
                Ok(())
            }
        };

        assert_eq!(result_i, expected_i);
    }
}
