use syn::parse_quote;

use crate::configs::DimensionConfig;
use crate::utils::{init_resolution_ident, operon_ident};

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

        if self.0.depends_on.is_empty() {
            writeln!(f)?;
            return write!(f, ");");
        }

        writeln!(f, ",")?;
        writeln!(f, "    PRIMARY KEY ({})", self.0.depends_on.join(","))?;
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
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{dimension_i, dimension_j};

    #[rstest]
    #[case::simple(
        dimension_i(),
        indoc! {"
            CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_i (
                i_ub BIGINT NOT NULL
            );"
        },
    )]
    #[case::with_dependency(
        dimension_j(),
        indoc! {"
            CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_j (
                i BIGINT,
                j_ub BIGINT NOT NULL,
                PRIMARY KEY (i)
            );"
        }
    )]
    fn test_init_resolution_query_no_dependency(
        #[case] dim: DimensionConfig,
        #[case] expected: &str,
    ) {
        let stmt = InitResolutionQuery(&dim).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(dimension_i(), "queries/resolution/init_resolution.rs")]
    fn test_fn_init_resolution(#[case] dim: DimensionConfig, #[case] fixture_path: &str) {
        let result = fn_init_resolution(&dim);
        assert_item_eq(&result, fixture_path);
    }
}
