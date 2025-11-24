use quote::quote;
use syn::parse_quote;

use crate::configs::{EntityConfig, EntityConfigMap, JobArg, JobConfigMap};
use crate::utils::{batch_get_entity_ident, entity_ident, operon_ident, variable_ident};

struct BatchGetQuery<'a>(&'a JobArg, &'a EntityConfig);

impl std::fmt::Display for BatchGetQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SELECT value")?;
        for over_dim in &self.0.over {
            write!(f, ", {over_dim}")?;
        }
        writeln!(f)?;
        writeln!(f, "FROM {{schema_prefix}}{}", self.0.id)?;
        for (idx, dim) in self
            .1
            .dims
            .iter()
            .filter(|dim| !self.0.over.contains(dim))
            .enumerate()
        {
            if idx == 0 {
                write!(f, "WHERE")?;
            } else {
                write!(f, " AND")?;
            }
            write!(f, " {} = ${}", dim, idx + 1)?;
        }
        writeln!(f)?;
        for (idx, dim) in self.0.over.iter().enumerate() {
            if idx == 0 {
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
        let arg_id = entity_ident(&arg.id);
        let batch_get_query = BatchGetQuery(arg, arg_config).to_string();

        let entity_ident = entity_ident(&arg.id);
        let return_ty: syn::Type = arg.over.iter().fold(
            parse_quote! { #entity_ident },
            |acc, _| parse_quote! { Vec<#acc> },
        );

        let arg_dims = arg_config.dims.iter().filter(|d| !arg.over.contains(d)).collect::<Vec<_>>();
        let args = arg_dims.iter().map(|d| variable_ident(d)).collect::<Vec<_>>();

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
            async fn #batch_get_fn_name(&self, #(#args: usize),*) -> Result<#return_ty, #operon::storage::StorageError> {
                let conn = self.pool.get().await?;
                let schema_prefix = #operon::utils::SchemaPrefix(self.schema.as_deref());
                let stmt = format!(#batch_get_query);
                let rows = conn
                    .query(&stmt, &[#(&i64::try_from(#args)?),*])
                    .await?;

                let mut result: #return_ty = Default::default();
                for row in rows {
                    let value = #operon::serde_json::from_value::<#arg_id>(row.get(0))?;
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
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_items_eq_in_trait;
    use crate::test_utils::simple_pipeline::{all_entities, all_jobs, entity_d};

    #[rstest]
    #[case::simple(
        JobArg {
            id: "d".to_string(),
            over: vec!["j".to_string()],
        },
        entity_d(),
        indoc! {"
            SELECT value, j
            FROM {schema_prefix}d
            WHERE i = $1 AND k = $2
            ORDER BY j"
        },
    )]
    fn test_batch_get_query(
        #[case] job_arg: JobArg,
        #[case] entity: EntityConfig,
        #[case] expected: &str,
    ) {
        let stmt = BatchGetQuery(&job_arg, &entity).to_string();
        assert_eq!(stmt, expected);
    }

    #[rstest]
    fn test_batch_gets(all_jobs: JobConfigMap, all_entities: EntityConfigMap) {
        let items = batch_gets(&all_jobs, &all_entities).collect::<Vec<_>>();
        assert_items_eq_in_trait(&items, "storage/batch_gets.rs");
    }
}
