use syn::parse_quote;

use crate::configs::DimensionConfig;
use crate::utils::{
    dimension_ident, get_resolution_ident, operon_ident, resolution_ident, variable_ident,
};

/// Helper struct to generate the SQL query for getting a dimension's resolution.
struct GetResolutionQuery<'a>(&'a DimensionConfig);

impl std::fmt::Display for GetResolutionQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SELECT {}_ub FROM {{schema_prefix}}dimension_{}",
            self.0.id, self.0.id
        )?;

        for (idx, dim) in self.0.depends_on.iter().enumerate() {
            if idx == 0 {
                write!(f, " WHERE")?;
            } else {
                write!(f, " AND")?;
            }
            write!(f, " {} = ${}", dim, idx + 1)?;
        }
        write!(f, ";")
    }
}

/// Generates the `get_resolution_*` function for a given dimension.
///
/// Example:
/// ```rust,ignore
/// pub async fn get_resolution_i(
///    client: operon::meta_storage::MetaClient<'_>,
/// ) -> Result<Option<schema::IResolution>, operon::meta_storage::MetaStorageError> {
///     let schema_prefix = client.schema_prefix();
///     let stmt = format!("SELECT i_ub FROM {schema_prefix}dimension_i;");
///     let Some(row) = client.query_opt(&stmt, &[]).await? else {
///         return Ok(None);
///     };
///     Ok(Some(schema::IResolution(
///         usize::try_from(row.get::<_, i64>("i_ub"))?,
///     )))
/// }
/// ```
pub(super) fn fn_get_resolution(dimension: &DimensionConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = get_resolution_ident(&dimension.id);
    let res_ident = resolution_ident(&dimension.id);
    let stmt = GetResolutionQuery(dimension).to_string();

    let deps = &dimension
        .depends_on
        .iter()
        .map(|d| variable_ident(d))
        .collect::<Vec<_>>();
    let args = dimension.depends_on.iter().map(|d| -> syn::FnArg {
        let arg = variable_ident(d);
        let dim_ident = dimension_ident(d);
        parse_quote! { #arg: schema::#dim_ident }
    });
    let ub_id = format!("{}_ub", dimension.id);

    parse_quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>,
            #(#args,)*
        ) -> Result<Option<schema::#res_ident>, #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(#stmt);
            let Some(row) = client.query_opt(&stmt, &[#(&i64::try_from(#deps)?),*]).await? else {
                return Ok(None);
            };
            Ok(Some(schema::#res_ident(
                usize::try_from(row.get::<_, i64>(#ub_id))?,
                #(#deps,)*
            )))
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
    #[case::simple(dimension_i(), "SELECT i_ub FROM {schema_prefix}dimension_i;")]
    #[case::with_dependency(
        dimension_j(),
        "SELECT j_ub FROM {schema_prefix}dimension_j WHERE i = $1;"
    )]
    fn test_get_resolution_query(#[case] dim: DimensionConfig, #[case] expected: &str) {
        let stmt = GetResolutionQuery(&dim).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    #[case::simple(dimension_i(), "queries/resolution/get_resolution.simple.rs")]
    #[case::with_dependency(dimension_j(), "queries/resolution/get_resolution.with_dependency.rs")]
    fn test_get_resolution(#[case] dim: DimensionConfig, #[case] fixture_path: &str) {
        let result = fn_get_resolution(&dim);
        assert_item_eq(&result, fixture_path);
    }
}
