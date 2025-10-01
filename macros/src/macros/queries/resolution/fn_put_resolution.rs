use quote::format_ident;
use syn::parse_quote;

use crate::configs::DimensionConfig;
use crate::utils::{operon_ident, resolution_ident};

/// Helper struct to generate the SQL query for inserting a resolution into a dimension.
pub struct PutResolutionQuery<'a>(pub(super) &'a DimensionConfig);

impl std::fmt::Display for PutResolutionQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "INSERT INTO {{schema_prefix}}dimension_{} (", self.0.id)?;
        for dep in &self.0.depends_on {
            write!(f, "{dep}, ")?;
        }
        write!(f, "{}_ub) VALUES ($1", self.0.id)?;
        for i in 1..(self.0.depends_on.len() + 1) {
            write!(f, ", ${}", i + 1)?;
        }
        write!(f, ") ON CONFLICT DO NOTHING;")?;

        Ok(())
    }
}

/// Generates the `put_resolution_*` function for a given dimension.
///
/// Example:
/// ```rust,ignore
/// pub async fn put_resolution_i(
///     client: operon::meta_storage::MetaClient<'_>,
///     resolution: &schema::IResolution,
/// ) -> Result<(), operon::meta_storage::MetaStorageError> {
///     let schema_prefix = client.schema_prefix();
///     let stmt = format!("INSERT INTO {schema_prefix}dimension_i (i_ub) VALUES ($1) ON CONFLICT DO NOTHING;");
///     client.execute(&stmt, &[&i64::try_from(resolution.0)?]).await?;
///     Ok(())
/// }
/// ```
pub(super) fn fn_put_resolution(dimension: &DimensionConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = format_ident!("put_resolution_{}", dimension.id);
    let res_ident = resolution_ident(&dimension.id);
    let stmt = PutResolutionQuery(dimension).to_string();

    let indices = (1..=dimension.depends_on.len()).map(syn::Index::from);

    parse_quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>,
            resolution: &schema::#res_ident,
        ) -> Result<(), #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(#stmt);
            client.execute(&stmt, &[#(&i64::try_from(resolution.#indices)?,)* &i64::try_from(resolution.0)?]).await?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_resolution_query_no_dependency() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let put_resolution = PutResolutionQuery(&dimension_i);

        let stmt_i =
            "INSERT INTO {schema_prefix}dimension_i (i_ub) VALUES ($1) ON CONFLICT DO NOTHING;";

        assert_eq!(put_resolution.to_string(), stmt_i);
    }

    #[test]
    fn test_put_resolution_query_with_dependency() {
        let dimension_j = DimensionConfig {
            id: "j".to_string(),
            depends_on: vec!["i".to_string()],
        };

        let put_resolution = PutResolutionQuery(&dimension_j);

        let stmt_j = "INSERT INTO {schema_prefix}dimension_j (i, j_ub) VALUES ($1, $2) ON CONFLICT DO NOTHING;";

        assert_eq!(put_resolution.to_string(), stmt_j);
    }

    #[test]
    fn test_put_resolution() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let result_i = fn_put_resolution(&dimension_i);

        let stmt_i =
            "INSERT INTO {schema_prefix}dimension_i (i_ub) VALUES ($1) ON CONFLICT DO NOTHING;"; // Note: placing this string literal inside the quote! macro results in a `\n` instead of `\\n`, causing the test to fail.

        let expected_i: syn::ItemFn = parse_quote! {
            pub async fn put_resolution_i(
                client: operon::meta_storage::MetaClient<'_>,
                resolution: &schema::IResolution,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                let schema_prefix = client.schema_prefix();
                let stmt = format!(#stmt_i);
                client.execute(&stmt, &[&i64::try_from(resolution.0)?]).await?;
                Ok(())
            }
        };

        assert_eq!(result_i, expected_i);

        let dimension_l = DimensionConfig {
            id: "l".to_string(),
            depends_on: vec!["j".to_string(), "k".to_string()],
        };

        let result_l = fn_put_resolution(&dimension_l);
        let stmt_l = "INSERT INTO {schema_prefix}dimension_l (j, k, l_ub) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING;";
        let expected_l: syn::ItemFn = parse_quote! {
            pub async fn put_resolution_l(
                client: operon::meta_storage::MetaClient<'_>,
                resolution: &schema::LResolution,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                let schema_prefix = client.schema_prefix();
                let stmt = format!(#stmt_l);
                client.execute(&stmt, &[&i64::try_from(resolution.1)?, &i64::try_from(resolution.2)?, &i64::try_from(resolution.0)?]).await?;
                Ok(())
            }
        };

        assert_eq!(result_l, expected_l);
    }
}
