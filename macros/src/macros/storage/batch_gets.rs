use quote::quote;
use syn::parse_quote;

use crate::{
    EntityConfig, JobArg, JobConfigMap,
    configs::EntityConfigMap,
    utils::{batch_get_entity_ident, dimension_ident, entity_ident, operon_ident, variable_ident},
};

struct BatchGetQuery<'a>(&'a JobArg, &'a EntityConfig);

impl std::fmt::Display for BatchGetQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SELECT value")?;
        for over_dim in &self.0.over {
            write!(f, ", {over_dim}")?;
        }
        writeln!(f)?;
        writeln!(f, "FROM {{schema_prefix}}{}", self.0.id)?;
        for (i, dim) in self
            .1
            .dims
            .iter()
            .filter(|dim| !self.0.over.contains(dim))
            .enumerate()
        {
            if i == 0 {
                write!(f, "WHERE")?;
            } else {
                write!(f, " AND")?;
            }
            write!(f, " {} = ${}", dim, i + 1)?;
        }
        writeln!(f)?;
        for (i, dim) in self.0.over.iter().enumerate() {
            if i == 0 {
                write!(f, "ORDER BY {dim}")?;
            } else {
                write!(f, ", {dim}")?;
            }
        }
        Ok(())
    }
}

/// A helper function to generate batch get functions for each job.
pub fn batch_gets(
    jobs: &JobConfigMap,
    entities: &EntityConfigMap,
) -> impl Iterator<Item = syn::TraitItemFn> {
    let mut targets = jobs
        .values()
        .flat_map(|job| job.from.iter().filter(|arg| !arg.over.is_empty()))
        .collect::<Vec<_>>();

    targets.sort_by_key(|arg| arg.id.as_str());
    targets.dedup_by_key(|arg| arg.id.as_str());

    targets.into_iter().map(|arg| -> syn::TraitItemFn {
        let operon = operon_ident();
        let batch_get_fn_name = batch_get_entity_ident(&arg.id, &arg.over);

        let arg_config = entities.get(&arg.id)
            .unwrap_or_else(|| panic!("Entity {} not found in entities", arg.id));
        let generic = &arg_config.generic;
        let batch_get_query = BatchGetQuery(arg, arg_config).to_string();

        let entity_ident = entity_ident(&arg.id);
        let return_ty: syn::Type = arg.over.iter().fold(
            parse_quote! { #entity_ident },
            |acc, _| parse_quote! { Vec<#acc> },
        );

        let arg_dims = arg_config.dims.iter().filter(|d| !arg.over.contains(d)).collect::<Vec<_>>();
        let fn_args = arg_dims.iter().map(|d| -> syn::FnArg {
            let arg_ident = variable_ident(d);
            let arg_ty = dimension_ident(d);
            parse_quote! { #arg_ident: schema::#arg_ty }
        });
        let query_params = arg_dims.iter().map(|d| variable_ident(d));

        let insert_results = arg.over.iter().enumerate().map(|(i, d)| {
            let dim_var = variable_ident(d);
            let i_plus_1 = i + 1;

            if i_plus_1 == arg.over.len() {
                quote! {
                    result.push(value.into());
                }
            } else {
                quote! {
                    let #dim_var = usize::try_from(row.get::<_, i64>(#i_plus_1))?; // TODO: Handle None case
                    while result.len() <= #dim_var {
                        result.push(Default::default());
                    }
                    let mut result = &mut result[#dim_var];
                }
            }
        });

        parse_quote! {
            async fn #batch_get_fn_name(&self, #(#fn_args),*) -> Result<#return_ty, #operon::storage::StorageError> {
                let conn = self.pool.get().await?;
                let schema_prefix = #operon::utils::SchemaPrefix(self.schema.as_deref());
                let stmt = format!(#batch_get_query);
                let rows = conn
                    .query(&stmt, &[#(&i64::try_from(#query_params)?),*])
                    .await?;

                let mut result: #return_ty = Default::default();
                for row in rows {
                    let value = #operon::serde_json::from_value::<#generic>(row.get(0))?;
                    #(#insert_results)*
                }
                Ok(result)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use std::vec;

    use indoc::indoc;
    use quote::format_ident;

    use crate::JobConfig;

    use super::*;

    #[test]
    fn test_batch_get_query() {
        let job_arg = JobArg {
            id: "d".to_string(),
            over: vec!["j".to_string()],
        };
        let entity_config = EntityConfig {
            id: "d".to_string(),
            dims: vec!["i".to_string(), "j".to_string(), "k".to_string()],
            generic: parse_quote!(String),
        };

        let query = BatchGetQuery(&job_arg, &entity_config).to_string();
        let expected = indoc! {"
            SELECT value, j
            FROM {schema_prefix}d
            WHERE i = $1 AND k = $2
            ORDER BY j"
        };
        assert_eq!(query, expected);
    }

    #[test]
    fn test_batch_gets() {
        let entities = EntityConfigMap::from_iter([
            (
                "b".to_string(),
                EntityConfig {
                    id: "b".to_string(),
                    dims: vec!["i".to_string(), "j".to_string()],
                    generic: format_ident!("B_"),
                },
            ),
            (
                "d".to_string(),
                EntityConfig {
                    id: "d".to_string(),
                    dims: vec!["i".to_string(), "j".to_string(), "k".to_string()],
                    generic: format_ident!("D_"),
                },
            ),
            (
                "e".to_string(),
                EntityConfig {
                    id: "e".to_string(),
                    dims: vec!["i".to_string(), "k".to_string()],
                    generic: format_ident!("E_"),
                },
            ),
        ]);
        let jobs = JobConfigMap::from_iter([(
            "epsilon".to_string(),
            JobConfig {
                id: "epsilon".to_string(),
                from: vec![
                    JobArg {
                        id: "b".to_string(),
                        over: vec!["j".to_string()],
                    },
                    JobArg {
                        id: "d".to_string(),
                        over: vec!["j".to_string()],
                    },
                ],
                to: "e".to_string(),
                dims: vec!["i".to_string(), "k".to_string()],
                spawn_dim: None,
                pool_size: 4,
            },
        )]);

        let item = batch_gets(&jobs, &entities).collect::<Vec<_>>();
        let expected: Vec<syn::TraitItemFn> = vec![
            parse_quote! {
                async fn get_all_b_over_j(&self, i: schema::IDim) -> Result<Vec<B>, operon::storage::StorageError> {
                    let conn = self.pool.get().await?;
                    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());
                    let stmt = format!("SELECT value, j\nFROM {schema_prefix}b\nWHERE i = $1\nORDER BY j");
                    let rows = conn.query(&stmt, &[&i64::try_from(i)?]).await?;

                    let mut result: Vec<B> = Default::default();
                    for row in rows {
                        let value = operon::serde_json::from_value::<B_>(row.get(0))?;
                        result.push(value.into());
                    }

                    Ok(result)
                }
            },
            parse_quote! {
                async fn get_all_d_over_j(&self, i: schema::IDim, k: schema::KDim) -> Result<Vec<D>, operon::storage::StorageError> {
                    let conn = self.pool.get().await?;
                    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());
                    let stmt = format!("SELECT value, j\nFROM {schema_prefix}d\nWHERE i = $1 AND k = $2\nORDER BY j");
                    let rows = conn.query(&stmt, &[&i64::try_from(i)?, &i64::try_from(k)?]).await?;

                    let mut result: Vec<D> = Default::default();
                    for row in rows {
                        let value = operon::serde_json::from_value::<D_>(row.get(0))?;
                        result.push(value.into());
                    }

                    Ok(result)
                }
            },
        ];
        assert_eq!(item, expected);
    }

    #[test]
    fn test_batch_get_over_multiple_dimension() {
        let entities = EntityConfigMap::from_iter([
            (
                "b".to_string(),
                EntityConfig {
                    id: "b".to_string(),
                    dims: vec!["i".to_string(), "j".to_string()],
                    generic: format_ident!("B_"),
                },
            ),
            (
                "d".to_string(),
                EntityConfig {
                    id: "d".to_string(),
                    dims: vec!["i".to_string(), "j".to_string(), "k".to_string()],
                    generic: format_ident!("D_"),
                },
            ),
            (
                "e".to_string(),
                EntityConfig {
                    id: "e".to_string(),
                    dims: vec!["i".to_string(), "l".to_string()],
                    generic: format_ident!("E_"),
                },
            ),
        ]);
        let jobs = JobConfigMap::from_iter([(
            "epsilon".to_string(),
            JobConfig {
                id: "epsilon".to_string(),
                from: vec![
                    JobArg {
                        id: "b".to_string(),
                        over: vec!["j".to_string()],
                    },
                    JobArg {
                        id: "d".to_string(),
                        over: vec!["j".to_string(), "k".to_string()],
                    },
                ],
                to: "e".to_string(),
                dims: vec!["i".to_string()],
                spawn_dim: Some("l".to_string()),
                pool_size: 4,
            },
        )]);

        let item = batch_gets(&jobs, &entities).collect::<Vec<_>>();
        let expected: Vec<syn::TraitItemFn> = vec![
            parse_quote! {
                async fn get_all_b_over_j(&self, i: schema::IDim) -> Result<Vec<B>, operon::storage::StorageError> {
                    let conn = self.pool.get().await?;
                    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());
                    let stmt = format!("SELECT value, j\nFROM {schema_prefix}b\nWHERE i = $1\nORDER BY j");
                    let rows = conn.query(&stmt, &[&i64::try_from(i)?]).await?;

                    let mut result: Vec<B> = Default::default();
                    for row in rows {
                        let value = operon::serde_json::from_value::<B_>(row.get(0))?;
                        result.push(value.into());
                    }

                    Ok(result)
                }
            },
            parse_quote! {
                async fn get_all_d_over_jk(&self, i: schema::IDim) -> Result<Vec<Vec<D>>, operon::storage::StorageError> {
                    let conn = self.pool.get().await?;
                    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());
                    let stmt = format!("SELECT value, j, k\nFROM {schema_prefix}d\nWHERE i = $1\nORDER BY j, k");
                    let rows = conn.query(&stmt, &[&i64::try_from(i)?]).await?;

                    let mut result: Vec<Vec<D>> = Default::default();
                    for row in rows {
                        let value = operon::serde_json::from_value::<D_>(row.get(0))?;
                        let j = usize::try_from(row.get::<_, i64>(1usize))?;
                        while result.len() <= j {
                            result.push(Default::default());
                        }
                        let mut result = &mut result[j];
                        result.push(value.into());
                    }

                    Ok(result)
                }
            },
        ];
        assert_eq!(item, expected);
    }
}
