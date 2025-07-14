use syn::parse_quote;

use crate::{
    configs::DimensionConfig,
    utils::{init_resolution_ident, operon_ident},
};

/// Helper struct to generate the SQL query for initializing a dimension's resolution.
struct InitResolutionQuery<'a>(&'a DimensionConfig);

impl std::fmt::Display for InitResolutionQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "CREATE TABLE IF NOT EXISTS {{schema_prefix}}dimension_{} (",
            self.0.id
        )?;
        for dep in &self.0.depends_on {
            writeln!(f, "    {dep} BIGINT,")?;
        }
        write!(f, "    {}_ub BIGINT NOT NULL", self.0.id)?;
        if !self.0.depends_on.is_empty() {
            writeln!(f, ",")?;
            writeln!(f, "    PRIMARY KEY ({})", self.0.depends_on.join(","))?;
        } else {
            writeln!(f)?;
        }
        write!(f, ");")
    }
}

/// Generates the `init_resolution_*` function for a given dimension.
///
/// Example:
/// ```rust,ignore
/// pub async fn init_resolution_i(
///     client: operon::meta_storage::MetaClient<'_>
/// ) -> Result<(), operon::meta_storage::MetaStorageError> {
///     let schema_prefix = client.schema_prefix();
///     let stmt = format!("CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_i (i_ub BIGINT NOT NULL);");
///     client.execute(&stmt, &[]).await?;
///     Ok(())
/// }
/// ```
pub(super) fn fn_init_resolution(dimension: &DimensionConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = init_resolution_ident(&dimension.id);
    let stmt = InitResolutionQuery(dimension).to_string();

    parse_quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>
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
    use indoc::indoc;

    use super::*;

    #[test]
    fn test_init_resolution_query_no_dependency() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let init_resolution = InitResolutionQuery(&dimension_i);

        let stmt_i = indoc! {"
            CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_i (
                i_ub BIGINT NOT NULL
            );"
        }; // Note: placing this string literal inside the quote! macro results in a `\n` instead of `\\n`, causing the test to fail.

        assert_eq!(init_resolution.to_string(), stmt_i);
    }

    #[test]
    fn test_init_resolution_query_with_dependency() {
        let dimension_j = DimensionConfig {
            id: "j".to_string(),
            depends_on: vec!["i".to_string()],
        };

        let init_resolution = InitResolutionQuery(&dimension_j);

        let stmt_i = indoc! {"
            CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_j (
                i BIGINT,
                j_ub BIGINT NOT NULL,
                PRIMARY KEY (i)
            );"
        };

        assert_eq!(init_resolution.to_string(), stmt_i);
    }

    #[test]
    fn test_fn_init_resolution() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let result_i = fn_init_resolution(&dimension_i);

        let stmt_i = indoc! {"
            CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_i (
                i_ub BIGINT NOT NULL
            );"
        }; // Note: placing this string literal inside the quote! macro results in a `\n` instead of `\\n`, causing the test to fail.
        let expected_i: syn::ItemFn = parse_quote! {
            pub async fn init_resolution_i(
                client: operon::meta_storage::MetaClient<'_>
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
