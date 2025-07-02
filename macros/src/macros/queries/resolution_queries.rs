use quote::{format_ident, quote};

use crate::{
    configs::DimensionConfig,
    utils::{
        clear_resolution_ident, get_resolution_ident, init_resolution_ident, operon_ident,
        resolution_ident,
    },
};

/// Generates the `init_resolution_*` function for a given dimension.
///
/// Example:
/// ```
/// pub async fn init_resolution_i(
///     client: operon::meta_storage::MetaClient<'_>
/// ) -> Result<(), operon::meta_storage::MetaStorageError> {
///     let schema_prefix = client.schema_prefix();
///     let stmt = format!("CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_i (i_ub BIGINT NOT NULL);");
///     client.execute(&stmt, &[]).await?;
///     Ok(())
/// }
fn fn_init_resolution(dimension: &DimensionConfig) -> proc_macro2::TokenStream {
    let operon = operon_ident();
    let fn_name = init_resolution_ident(&dimension.id);
    let stmt = dimension.init_resolution_query().to_string();

    quote! {
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

/// Generates the `clear_resolution_*` function for a given dimension.
///
/// Example:
/// ```
/// pub async fn clear_resolution_i(
///   client: operon::meta_storage::MetaClient<'_>,
/// ) -> Result<(), operon::meta_storage::MetaStorageError> {
///     let schema_prefix = client.schema_prefix();
///     
///     let stmt = format!("TRUNCATE TABLE {schema_prefix}dimension_i;");
///     client.execute(&stmt, &[]).await?;
///     Ok(())
/// }
fn fn_clear_resolution(dimension: &DimensionConfig) -> proc_macro2::TokenStream {
    let operon = operon_ident();
    let fn_name = clear_resolution_ident(&dimension.id);
    let stmt = dimension.clear_resolution_query().to_string();

    quote! {
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

/// Generates the `get_resolution_*` function for a given dimension.
///
/// Example:
/// ```
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
fn fn_get_resolution(dimension: &DimensionConfig) -> proc_macro2::TokenStream {
    let operon = operon_ident();
    let fn_name = get_resolution_ident(&dimension.id);
    let resolution_ident = resolution_ident(&dimension.id);
    let stmt = dimension.get_resolution_query().to_string();

    let deps = &dimension
        .depends_on
        .iter()
        .map(|d| format_ident!("{d}"))
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

/// Generates the `put_resolution_*` function for a given dimension.
///
/// Example:
/// ```
/// pub async fn put_resolution_i(
///     client: operon::meta_storage::MetaClient<'_>,
///     resolution: &IResolution,
/// ) -> Result<(), operon::meta_storage::MetaStorageError> {
///     let schema_prefix = client.schema_prefix();
///     let stmt = format!("INSERT INTO {schema_prefix}dimension_i (i_ub) VALUES ($1) ON CONFLICT DO NOTHING;");
///     client.execute(&stmt, &[&i64::try_from(resolution.0)?]).await?;
///     Ok(())
/// }
fn fn_put_resolution(dimension: &DimensionConfig) -> proc_macro2::TokenStream {
    let operon = operon_ident();
    let fn_name = format_ident!("put_resolution_{}", dimension.id);
    let resolution_ident = resolution_ident(&dimension.id);
    let stmt = dimension.put_resolution_query().to_string();

    let indices = (1..=dimension.depends_on.len()).map(syn::Index::from);

    quote! {
        pub async fn #fn_name(
            client: #operon::meta_storage::MetaClient<'_>,
            resolution: &#resolution_ident,
        ) -> Result<(), #operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(#stmt);
            client.execute(&stmt, &[#(&i64::try_from(resolution.#indices)?,)* &i64::try_from(resolution.0)?]).await?;
            Ok(())
        }
    }
}

/// Generates all resolution-related queries for a given dimension.
pub fn resolution_queries(dimension: &DimensionConfig) -> proc_macro2::TokenStream {
    let init_fn = fn_init_resolution(dimension);
    let clear_fn = fn_clear_resolution(dimension);
    let get_fn = fn_get_resolution(dimension);
    let put_fn = fn_put_resolution(dimension);

    quote! {
        #init_fn
        #clear_fn
        #get_fn
        #put_fn
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    #[test]
    fn test_init_resolution() {
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
        let expected_i = quote! {
            pub async fn init_resolution_i(
                client: operon::meta_storage::MetaClient<'_>
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                let schema_prefix = client.schema_prefix();
                let stmt = format!(#stmt_i);
                client.execute(&stmt, &[]).await?;
                Ok(())
            }
        };

        assert_eq!(result_i.to_string(), expected_i.to_string());
    }

    #[test]
    fn test_clear_resolution() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let result_i = fn_clear_resolution(&dimension_i);

        let stmt_i = "TRUNCATE TABLE {schema_prefix}dimension_i;"; // Note: placing this string literal inside the quote! macro results in a `\n` instead of `\\n`, causing the test to fail.
        let expected_i = quote! {
            pub async fn clear_resolution_i(
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                let schema_prefix = client.schema_prefix();
                let stmt = format!(#stmt_i);
                client.execute(&stmt, &[]).await?;
                Ok(())
            }
        };

        assert_eq!(result_i.to_string(), expected_i.to_string());
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

    #[test]
    fn test_put_resolution() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let result_i = fn_put_resolution(&dimension_i);

        let stmt_i =
            "INSERT INTO {schema_prefix}dimension_i (i_ub) VALUES ($1) ON CONFLICT DO NOTHING;"; // Note: placing this string literal inside the quote! macro results in a `\n` instead of `\\n`, causing the test to fail.

        let expected_i = quote! {
            pub async fn put_resolution_i(
                client: operon::meta_storage::MetaClient<'_>,
                resolution: &IResolution,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                let schema_prefix = client.schema_prefix();
                let stmt = format!(#stmt_i);
                client.execute(&stmt, &[&i64::try_from(resolution.0)?]).await?;
                Ok(())
            }
        };

        assert_eq!(result_i.to_string(), expected_i.to_string());

        let dimension_l = DimensionConfig {
            id: "l".to_string(),
            depends_on: vec!["j".to_string(), "k".to_string()],
        };

        let result_l = fn_put_resolution(&dimension_l);
        let stmt_l = "INSERT INTO {schema_prefix}dimension_l (j, k, l_ub) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING;";
        let expected_l = quote! {
            pub async fn put_resolution_l(
                client: operon::meta_storage::MetaClient<'_>,
                resolution: &LResolution,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                let schema_prefix = client.schema_prefix();
                let stmt = format!(#stmt_l);
                client.execute(&stmt, &[&i64::try_from(resolution.1)?, &i64::try_from(resolution.2)?, &i64::try_from(resolution.0)?]).await?;
                Ok(())
            }
        };

        assert_eq!(result_l.to_string(), expected_l.to_string());
    }
}
