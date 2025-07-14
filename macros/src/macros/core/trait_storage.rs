use quote::{format_ident, quote};
use syn::parse_quote;

use crate::{
    AllConfig, JobConfigMap,
    configs::EntityConfigMap,
    utils::{
        batch_get_entity_ident, batch_put_entity_ident, dimension_ident, entity_ident,
        get_entity_ident, operon_ident, put_entity_ident, storage_trait_ident, variable_ident,
    },
};

/// A helper function to generate single operation functions for each entity.
fn single_ops(entities: &EntityConfigMap) -> impl Iterator<Item = syn::TraitItemFn> {
    entities.values().flat_map(|entity| -> [syn::TraitItemFn; 2] {
        let get_fn_name = get_entity_ident(&entity.id);
        let put_fn_name = put_entity_ident(&entity.id);
        let entity_ident = entity_ident(&entity.id);
        let dim_args = entity
            .dims
            .iter()
            .map(|d| -> syn::FnArg {
                let arg_ident = variable_ident(d);
                let arg_ty = dimension_ident(d);

                parse_quote! { #arg_ident: schema::#arg_ty }
            })
            .collect::<Vec<_>>();

        let get_fn = parse_quote! {
            async fn #get_fn_name(&self, #(#dim_args),*) -> Result<Option<#entity_ident>, operon::storage::StorageError>;
        };
        let put_fn = parse_quote! {
            async fn #put_fn_name(&self, #(#dim_args,)* value: #entity_ident) -> Result<(), operon::storage::StorageError>;
        };

        [get_fn, put_fn]
    })
}

/// A helper function to generate batch get functions for each job.
fn batch_gets(
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

        let arg_config = entities.get(&arg.id)
            .unwrap_or_else(|| panic!("Entity {} not found in entities", arg.id));

        let entity_ident = entity_ident(&arg.id);
        let return_ty: syn::Type = arg.over.iter().fold(
            parse_quote! { #entity_ident },
            |acc, _| parse_quote! { Vec<#acc> },
        );
        let dim_args = arg_config.dims.iter().filter_map(|d| -> Option<syn::FnArg> {
            if arg.over.contains(d) {
                return None;
            }

            let arg_ident = variable_ident(d);
            let arg_ty = dimension_ident(d);
            Some(parse_quote! { #arg_ident: schema::#arg_ty })
        });
        let dim_vars = arg_config.dims.iter().map(|d| -> syn::Expr {
            let var_ident = variable_ident(d);
            if arg.over.contains(d) {
                parse_quote! { #var_ident.into() }
            } else {
                parse_quote! { #var_ident }
            }
        });
        let batch_get_fn_name = batch_get_entity_ident(&arg.id, &arg.over);
        let get_fn_name = get_entity_ident(&arg.id);

        let body = arg.over.iter().enumerate().rfold(
            quote! {
                self.#get_fn_name(#(#dim_vars),*).await?
            },
            |acc, (i, over)| {
                let results_ident = format_ident!("results_{i}");
                let over_var = variable_ident(over);

                quote! {
                    {
                        let mut #results_ident = Vec::new();
                        let mut #over_var = 0usize;
                        while let Some(value) = #acc {
                            #results_ident.push(value);
                            #over_var += 1;
                        }
                        (!#results_ident.is_empty()).then_some(#results_ident)
                    }
                }
            }
        );

        parse_quote! {
            async fn #batch_get_fn_name(&self, #(#dim_args),*) -> Result<#return_ty, #operon::storage::StorageError> {
                let final_results = #body;
                Ok(final_results.unwrap_or_default())
            }
        }
    })
}

/// A helper function to generate batch insert functions for each job.
fn batch_inserts(jobs: &JobConfigMap) -> impl Iterator<Item = syn::TraitItemFn> {
    jobs.values().filter_map(|job| -> Option<syn::TraitItemFn> {
        let operon = operon_ident();
        let spawn_dim = job.spawn_dim.as_ref()?;

        let entity_ident = entity_ident(&job.to);
        let dim_args = job.dims.iter()
            .map(|d| -> syn::FnArg {
                let arg_ident = variable_ident(d);
                let arg_ty = dimension_ident(d);
                parse_quote! { #arg_ident: schema::#arg_ty }
            });
        let dim_vars = job.dims.iter().map(|d| variable_ident(d));
        let spawn_dim_var = variable_ident(spawn_dim);
        let batch_put_fn_name = batch_put_entity_ident(&job.to);
        let put_fn_name = put_entity_ident(&job.to);

        Some(parse_quote! {
            async fn #batch_put_fn_name(&self, #(#dim_args,)* values: Vec<#entity_ident>) -> Result<(), #operon::storage::StorageError> {
                for (#spawn_dim_var, value) in values.into_iter().enumerate() {
                    self.#put_fn_name(#(#dim_vars,)* #spawn_dim_var.into(), value).await?;
                }
                Ok(())
            }
        })
    })
}

/// Generates a trait for the storage required for the service.
///
/// Example:
/// ```rust,ignore
/// #[operon::async_trait::async_trait]
/// pub trait CookingStorage: operon::service::OperonService {
///     async fn get_a(&self, i: schema::IDim) -> Result<Option<A>, operon::storage::StorageError>;
///     async fn put_a(&self, i: schema::IDim, value: A) -> Result<(), operon::storage::StorageError>;
///
///     async fn get_b(&self, i: schema::IDim, j: schema::JDim) -> Result<Option<B>, operon::storage::StorageError>;
///     async fn put_b(&self, i: schema::IDim, j: schema::JDim, value: B) -> Result<(), operon::storage::StorageError>;
/// }
/// ```
pub fn trait_storage(all_configs: &AllConfig) -> syn::ItemTrait {
    let operon = operon_ident();
    let storage_ident = storage_trait_ident(&all_configs.service_id);

    let single_ops = single_ops(&all_configs.entities);
    let batch_gets = batch_gets(&all_configs.jobs, &all_configs.entities);
    let batch_inserts = batch_inserts(&all_configs.jobs);

    parse_quote! {
        #[#operon::async_trait::async_trait]
        pub trait #storage_ident: #operon::storage::OperonStorage {
            #(#single_ops)*
            #(#batch_gets)*
            #(#batch_inserts)*
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{EntityConfig, JobArg, JobConfig, JobConfigMap, configs::EntityConfigMap};

    use super::*;

    #[test]
    fn test_single_ops() {
        let entities = EntityConfigMap::from_iter([
            (
                "a".to_string(),
                EntityConfig {
                    id: "a".to_string(),
                    dims: vec!["i".to_string()],
                    generic: format_ident!("A_"),
                },
            ),
            (
                "b".to_string(),
                EntityConfig {
                    id: "b".to_string(),
                    dims: vec!["i".to_string(), "j".to_string()],
                    generic: format_ident!("B_"),
                },
            ),
        ]);
        let single_ops = single_ops(&entities).collect::<Vec<_>>();
        let expected: Vec<syn::TraitItemFn> = vec![
            parse_quote! {
                async fn get_a(&self, i: schema::IDim) -> Result<Option<A>, operon::storage::StorageError>;
            },
            parse_quote! {
                async fn put_a(&self, i: schema::IDim, value: A) -> Result<(), operon::storage::StorageError>;
            },
            parse_quote! {
                async fn get_b(&self, i: schema::IDim, j: schema::JDim) -> Result<Option<B>, operon::storage::StorageError>;
            },
            parse_quote! {
                async fn put_b(&self, i: schema::IDim, j: schema::JDim, value: B) -> Result<(), operon::storage::StorageError>;
            },
        ];

        assert_eq!(single_ops, expected);
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
            },
        )]);

        let item = batch_gets(&jobs, &entities).collect::<Vec<_>>();
        let expected: Vec<syn::TraitItemFn> = vec![
            parse_quote! {
                async fn get_all_b_over_j(&self, i: schema::IDim) -> Result<Vec<B>, operon::storage::StorageError> {
                    let final_results = {
                        let mut results_0 = Vec::new();
                        let mut j = 0usize;
                        while let Some(value) = self.get_b(i, j.into()).await? {
                            results_0.push(value);
                            j += 1;
                        }
                        (!results_0.is_empty()).then_some(results_0)
                    };
                    Ok(final_results.unwrap_or_default())
                }
            },
            parse_quote! {
                async fn get_all_d_over_j(&self, i: schema::IDim, k: schema::KDim) -> Result<Vec<D>, operon::storage::StorageError> {
                    let final_results = {
                        let mut results_0 = Vec::new();
                        let mut j = 0usize;
                        while let Some(value) = self.get_d(i, j.into(), k).await? {
                            results_0.push(value);
                            j += 1;
                        }
                        (!results_0.is_empty()).then_some(results_0)
                    };
                    Ok(final_results.unwrap_or_default())
                }
            },
        ];

        assert_eq!(item, expected);
    }

    #[test]
    fn test_batch_gets_with_multiple_over() {
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
            },
        )]);

        let item = batch_gets(&jobs, &entities).collect::<Vec<_>>();
        let expected: Vec<syn::TraitItemFn> = vec![
            parse_quote! {
                async fn get_all_b_over_j(&self, i: schema::IDim) -> Result<Vec<B>, operon::storage::StorageError> {
                    let final_results = {
                        let mut results_0 = Vec::new();
                        let mut j = 0usize;
                        while let Some(value) = self.get_b(i, j.into()).await? {
                            results_0.push(value);
                            j += 1;
                        }
                        (!results_0.is_empty()).then_some(results_0)
                    };
                    Ok(final_results.unwrap_or_default())
                }
            },
            parse_quote! {
                async fn get_all_d_over_jk(&self, i: schema::IDim) -> Result<Vec<Vec<D>>, operon::storage::StorageError> {
                    let final_results = {
                        let mut results_0 = Vec::new();
                        let mut j = 0usize;
                        while let Some(value) = {
                            let mut results_1 = Vec::new();
                            let mut k = 0usize;
                            while let Some(value) = self.get_d(i, j.into(), k.into()).await? {
                                results_1.push(value);
                                k += 1;
                            }
                            (!results_1.is_empty()).then_some(results_1)
                        }
                        {
                            results_0.push(value);
                            j += 1;
                        }
                        (!results_0.is_empty()).then_some(results_0)
                    };
                    Ok(final_results.unwrap_or_default())
                }
            },
        ];

        assert_eq!(item, expected);
    }

    #[test]
    fn test_batch_inserts() {
        let jobs = JobConfigMap::from_iter([
            (
                "beta".to_string(),
                JobConfig {
                    id: "beta".to_string(),
                    from: vec![JobArg {
                        id: "a".to_string(),
                        over: vec![],
                    }],
                    to: "b".to_string(),
                    dims: vec!["i".to_string()],
                    spawn_dim: Some("j".to_string()),
                },
            ),
            (
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
                },
            ),
        ]);

        let item = batch_inserts(&jobs).collect::<Vec<_>>();
        let expected: Vec<syn::TraitItemFn> = vec![parse_quote! {
            async fn put_all_b(&self, i: schema::IDim, values: Vec<B>) -> Result<(), operon::storage::StorageError> {
                for (j, value) in values.into_iter().enumerate() {
                    self.put_b(i, j.into(), value).await?;
                }
                Ok(())
            }
        }];

        assert_eq!(item, expected);
    }
}
