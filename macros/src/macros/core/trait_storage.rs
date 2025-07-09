use quote::{format_ident, quote};
use syn::parse_quote;

use crate::{
    AllConfig,
    utils::{
        batch_get_entity_ident, batch_put_entity_ident, dimension_ident, entity_ident,
        get_entity_ident, operon_ident, put_entity_ident, storage_trait_ident, variable_ident,
    },
};

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

    let single_ops = all_configs
        .entities
        .values()
        .flat_map(|entity| -> [syn::TraitItemFn; 2] {
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
                async fn #get_fn_name(&self, #(#dim_args),*) -> Result<Option<#entity_ident>, #operon::storage::StorageError>;
            };
            let put_fn = parse_quote! {
                async fn #put_fn_name(&self, #(#dim_args,)* value: &#entity_ident) -> Result<(), #operon::storage::StorageError>;
            };

            [get_fn, put_fn]
        });

    let batch_gets = all_configs.jobs.values().flat_map(|job| {
        job.from.iter().filter_map(|arg| -> Option<syn::TraitItemFn> {
            if arg.over.is_empty() {
                return None; // Skip if there are no dimensions to batch over
            }

            let arg_config = all_configs.entities.get(&arg.id)?;

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
                            for #over_var in 0.. {
                                match #acc {
                                    Some(value) => #results_ident.push(value),
                                    None => break,
                                }
                            }
                            (!#results_ident.is_empty()).then_some(#results_ident)
                        }
                    }
                }
            );

            Some(parse_quote! {
                async fn #batch_get_fn_name(&self, #(#dim_args),*) -> Result<#return_ty, #operon::storage::StorageError> {
                    let final_results = #body;
                    Ok(final_results.unwrap_or_default())
                }
            })
        })
    });

    let batch_inserts = all_configs.jobs.values().filter_map(|job| -> Option<syn::TraitItemFn> {
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
    });

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
    use quote::ToTokens;

    use crate::{EntityConfig, JobArg, JobConfig, JobConfigMap, configs::EntityConfigMap};

    use super::*;

    #[test]
    fn test_trait_storage() {
        let all_config = AllConfig {
            service_id: "Cooking".to_string(),
            primary_dimension: "i".to_string(),
            primary_entity: "a".to_string(),
            dimensions: Default::default(),
            entities: EntityConfigMap::from_iter([
                (
                    "a".to_string(),
                    EntityConfig {
                        id: "a".to_string(),
                        dims: vec!["i".to_string()],
                        body: String::new(),
                    },
                ),
                (
                    "b".to_string(),
                    EntityConfig {
                        id: "b".to_string(),
                        dims: vec!["i".to_string(), "j".to_string()],
                        body: String::new(),
                    },
                ),
                (
                    "c".to_string(),
                    EntityConfig {
                        id: "c".to_string(),
                        dims: vec!["i".to_string(), "k".to_string()],
                        body: String::new(),
                    },
                ),
                (
                    "d".to_string(),
                    EntityConfig {
                        id: "d".to_string(),
                        dims: vec!["i".to_string(), "j".to_string(), "k".to_string()],
                        body: String::new(),
                    },
                ),
                (
                    "e".to_string(),
                    EntityConfig {
                        id: "e".to_string(),
                        dims: vec!["i".to_string(), "k".to_string()],
                        body: String::new(),
                    },
                ),
                (
                    "f".to_string(),
                    EntityConfig {
                        id: "f".to_string(),
                        dims: vec!["i".to_string()],
                        body: String::new(),
                    },
                ),
            ]),
            jobs: JobConfigMap::from_iter([
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
            ]),
        };
        let item = trait_storage(&all_config);
        let expected: syn::ItemTrait = parse_quote! {
            #[operon::async_trait::async_trait]
            pub trait CookingStorage: operon::storage::OperonStorage {
                async fn get_a(&self, i: schema::IDim) -> Result<Option<A>, operon::storage::StorageError>;
                async fn put_a(&self, i: schema::IDim, value: &A) -> Result<(), operon::storage::StorageError>;

                async fn get_b(&self, i: schema::IDim, j: schema::JDim) -> Result<Option<B>, operon::storage::StorageError>;
                async fn put_b(&self, i: schema::IDim, j: schema::JDim, value: &B) -> Result<(), operon::storage::StorageError>;

                async fn get_c(&self, i: schema::IDim, k: schema::KDim) -> Result<Option<C>, operon::storage::StorageError>;
                async fn put_c(&self, i: schema::IDim, k: schema::KDim, value: &C) -> Result<(), operon::storage::StorageError>;

                async fn get_d(&self, i: schema::IDim, j: schema::JDim, k: schema::KDim) -> Result<Option<D>, operon::storage::StorageError>;
                async fn put_d(&self, i: schema::IDim, j: schema::JDim, k: schema::KDim, value: &D) -> Result<(), operon::storage::StorageError>;

                async fn get_e(&self, i: schema::IDim, k: schema::KDim) -> Result<Option<E>, operon::storage::StorageError>;
                async fn put_e(&self, i: schema::IDim, k: schema::KDim, value: &E) -> Result<(), operon::storage::StorageError>;

                async fn get_f(&self, i: schema::IDim) -> Result<Option<F>, operon::storage::StorageError>;
                async fn put_f(&self, i: schema::IDim, value: &F) -> Result<(), operon::storage::StorageError>;

                async fn get_all_b_over_j(&self, i: schema::IDim) -> Result<Vec<B>, operon::storage::StorageError> {
                    let final_results = {
                        let mut results_0 = Vec::new();
                        for j in 0.. {
                            match self.get_b(i, j.into()).await? {
                                Some(value) => results_0.push(value),
                                None => break,
                            }
                        }
                        (!results_0.is_empty()).then_some(results_0)
                    };
                    Ok(final_results.unwrap_or_default())
                }

                async fn get_all_d_over_j(&self, i: schema::IDim, k: schema::KDim) -> Result<Vec<D>, operon::storage::StorageError> {
                    let final_results = {
                        let mut results_0 = Vec::new();
                        for j in 0.. {
                            match self.get_d(i, j.into(), k).await? {
                                Some(value) => results_0.push(value),
                                None => break,
                            }
                        }
                        (!results_0.is_empty()).then_some(results_0)
                    };
                    Ok(final_results.unwrap_or_default())
                }

                async fn put_all_b(&self, i: schema::IDim, values: Vec<B>) -> Result<(), operon::storage::StorageError> {
                    for (j, value) in values.into_iter().enumerate() {
                        self.put_b(i, j.into(), value).await?;
                    }
                    Ok(())
                }

            }
        };

        assert_eq!(
            item.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
