use quote::quote;

use crate::{
    configs::DimensionConfig,
    utils::{dimension_ident, get_resolution_ident, operon_ident, resolution_ident},
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
        if let Some(dep) = self.0.depends_on.first() {
            write!(f, " WHERE {dep} = $1")?;
        }
        for (i, dep) in self.0.depends_on.iter().enumerate().skip(1) {
            write!(f, " AND {} = ${}", dep, i + 1)?;
        }

        Ok(())
    }
}

/// Generates the `get_resolution_*` function for a given dimension.
///
/// Example:
/// ```ignore
/// pub async fn get_resolution_i(
///    client: operon::meta_storage::MetaClient<'_>,
/// ) -> Result<Option<IResolution>, operon::meta_storage::MetaStorageError> {
///     let schema_prefix = client.schema_prefix();
///     let stmt = format!("SELECT i_ub FROM {schema_prefix}dimension_i");
///     let Some(row) = client.query_opt(&stmt, &[]).await? else {
///         return Ok(None);
///     };
///     Ok(IResolution(
///         usize::try_from(r.get::<_, i64>("i_ub"))?,
///     ))
/// }
/// ```
pub(super) fn fn_get_resolution(dimension: &DimensionConfig) -> proc_macro2::TokenStream {
    let operon = operon_ident();
    let fn_name = get_resolution_ident(&dimension.id);
    let resolution_ident = resolution_ident(&dimension.id);
    let stmt = GetResolutionQuery(dimension).to_string();

    let deps = &dimension
        .depends_on
        .iter()
        .map(|d| dimension_ident(d))
        .collect::<Vec<_>>();
    let ub_id = format!("{}_ub", dimension.id);

    quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>,
            #(#deps: usize,)*
        ) -> Result<Option<#resolution_ident>, #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(#stmt);
            let Some(row) = client.query_opt(&stmt, &[#(&i64::try_from(#deps)?),*]).await? else {
                return Ok(None);
            };
            Ok(#resolution_ident(
                usize::try_from(r.get::<_, i64>(#ub_id))?,
                #(#deps,)*
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_resolution_query_no_dependency() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let get_resolution = GetResolutionQuery(&dimension_i);

        let stmt_i = "SELECT i_ub FROM {schema_prefix}dimension_i";

        assert_eq!(get_resolution.to_string(), stmt_i);
    }

    #[test]
    fn test_get_resolution_query_with_dependency() {
        let dimension_l = DimensionConfig {
            id: "l".to_string(),
            depends_on: vec!["j".to_string(), "k".to_string()],
        };

        let get_resolution = GetResolutionQuery(&dimension_l);

        let stmt_l = "SELECT l_ub FROM {schema_prefix}dimension_l WHERE j = $1 AND k = $2";

        assert_eq!(get_resolution.to_string(), stmt_l);
    }

    #[test]
    fn test_get_resolution() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let result_i = fn_get_resolution(&dimension_i);

        let stmt_i = "SELECT i_ub FROM {schema_prefix}dimension_i"; // Note: placing this string literal inside the quote! macro results in a `\n` instead of `\\n`, causing the test to fail.

        let expected_i = quote! {
            pub async fn get_resolution_i(
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<Option<IResolution>, operon::meta_storage::MetaStorageError> {
                let schema_prefix = client.schema_prefix();
                let stmt = format!(#stmt_i);
                let row = client.query_opt(&stmt, &[]).await?;
                Ok(row.map(|r| {
                    IResolution(
                        usize::try_from(r.get::<_, i64>("i_ub"))?,
                    )
                }))
            }
        };

        assert_eq!(result_i.to_string(), expected_i.to_string());

        let dimension_l = DimensionConfig {
            id: "l".to_string(),
            depends_on: vec!["j".to_string(), "k".to_string()],
        };

        let result_l = fn_get_resolution(&dimension_l);
        let stmt_l = "SELECT l_ub FROM {schema_prefix}dimension_l WHERE j = $1 AND k = $2";
        let expected_l = quote! {
            pub async fn get_resolution_l(
                client: operon::meta_storage::MetaClient<'_>,
                j: usize,
                k: usize,
            ) -> Result<Option<LResolution>, operon::meta_storage::MetaStorageError> {
                let schema_prefix = client.schema_prefix();
                let stmt = format!(#stmt_l);
                let row = client.query_opt(&stmt, &[&i64::try_from(j)?, &i64::try_from(k)?]).await?;
                Ok(row.map(|r| {
                    LResolution(
                        usize::try_from(r.get::<_, i64>("l_ub"))?,
                        j,
                        k,
                    )
                }))
            }
        };

        assert_eq!(result_l.to_string(), expected_l.to_string());
    }
}
