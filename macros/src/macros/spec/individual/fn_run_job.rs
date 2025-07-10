use heck::ToSnakeCase;
use indexmap::{IndexMap, IndexSet};
use quote::{format_ident, quote};
use syn::parse_quote;

use crate::{
    DimensionConfig, JobConfig,
    configs::{DimensionConfigMap, DimensionId, EntityConfigMap},
    utils::{
        batch_get_entity_ident, batch_put_entity_ident, dimension_ident, entity_over_dim_ident,
        get_entity_ident, get_resolution_ident, job_fn_ident, operon_ident, put_entity_ident,
        resolution_ident, variable_ident,
    },
};

fn resolution_map_ident(dim: &DimensionId) -> syn::Ident {
    format_ident!("resolution_{}", dim.to_snake_case())
}

struct ResolutionHelp<'a> {
    config: &'a DimensionConfig,
    mapped_over: Vec<&'a DimensionId>,
}

pub(super) fn fn_run_job(
    job: &JobConfig,
    entities: &EntityConfigMap,
    dimensions: &DimensionConfigMap,
) -> syn::ImplItemFn {
    let operon = operon_ident();

    let args = job
        .from
        .iter()
        .map(|arg| entity_over_dim_ident(&arg.id, &arg.over));

    let job_dim_set: IndexSet<&DimensionId> = job.dims.iter().collect();
    // set of resolutions that needs to be fetched from storage, the values are extra dimensions which will be used as a key to the generated HashMap
    let resolution_index: IndexMap<&DimensionId, ResolutionHelp<'_>> = job
        .from
        .iter()
        .flat_map(|arg| {
            arg.over.iter().map(|over_dim_id| {
                let over_dim = dimensions.get(over_dim_id).unwrap_or_else(|| {
                    panic!("Dimension `{over_dim_id}` not found in config");
                });

                // Collect dependencies that are not part of the job dimensions, we need to fetch resolutions for these dimensions
                let mapped_over = over_dim
                    .depends_on
                    .iter()
                    .filter(|dep_dim| !job_dim_set.contains(dep_dim))
                    .collect::<Vec<_>>();

                let help = ResolutionHelp {
                    config: over_dim,
                    mapped_over,
                };

                (over_dim_id, help)
            })
        })
        .collect();

    let resolution_defs = resolution_index.iter().map(|(dim, help)| -> syn::Stmt {
        let res_map = resolution_map_ident(dim);
        let key_ty = help.mapped_over.iter().map(|dep| dimension_ident(dep));
        let ty: syn::Type =
            parse_quote! { std::collections::HashMap<(#(schema::#key_ty,)*), usize> };

        parse_quote! {
            let mut #res_map: #ty = Default::default();
        }
    });

    let resolution_inserts =
        resolution_index.iter().map(|(dim, help)| {
            let res_map = resolution_map_ident(dim);
            let get_resolution_fn_name = get_resolution_ident(dim);
            let get_resolution_args = help
                .config
                .depends_on
                .iter()
                .map(|dep| -> syn::Expr {
                    let dep_var = variable_ident(dep);
                    if job_dim_set.contains(dep) {
                        parse_quote! { job.#dep_var }
                    } else {
                        parse_quote! { #dep_var }
                    }
                })
                .collect::<Vec<_>>();
            let dep_vars = help.mapped_over.iter().map(|dep| variable_ident(dep));

            let missing_msg = format!(
                "{}_{}",
                dim,
                "{},"
                    .repeat(help.config.depends_on.len())
                    .trim_end_matches(",")
            );

            help.mapped_over.iter().rfold(
            quote! {
                let resolution = queries::#get_resolution_fn_name(client, #(#get_resolution_args),*)
                    .await?
                    .ok_or_else(|| {
                        #operon::meta_storage::MetaStorageError::MissingResolution(
                            format!(#missing_msg, #(#get_resolution_args),*)
                        )
                    })?;
                #res_map.insert((#(#dep_vars,)*), resolution.0);
            },
            |acc, dep| {
                let dep_var = variable_ident(dep);
                let dep_res_map = resolution_map_ident(dep);
                let dep_ub_key = resolution_index.get(dep).unwrap_or_else(|| {
                    panic!("Dimension `{dep}` not found in resolution index");
                }).mapped_over.iter().map(|d| variable_ident(d));

                quote! {
                    for #dep_var in 0..(*#dep_res_map.get(&(#(#dep_ub_key,)*)).unwrap_or(&0)) { // TODO: Remove unwrap
                        #acc
                    }
                }
            },
        )
        });

    let arg_defs = job.from.iter().map(|arg| -> syn::Stmt {
        let entity = entities.get(&arg.id).expect("Entity not found in config");

        let arg_ident = entity_over_dim_ident(&arg.id, &arg.over);

        let get_args = entity
            .dims
            .iter()
            .filter(|dim| !arg.over.contains(dim))
            .map(|dim| variable_ident(dim))
            .collect::<Vec<_>>();

        if arg.over.is_empty() {
            let get_ident = get_entity_ident(&arg.id);
            let not_found_msg = format!(
                "{}_{}",
                arg.id,
                "{},".repeat(entity.dims.len()).trim_end_matches(",")
            );
            return parse_quote! {
                let #arg_ident = storage
                    .#get_ident(#(job.#get_args),*)
                    .await?
                    .ok_or_else(|| {
                        #operon::storage::StorageError::NotFound(
                            format!(#not_found_msg, #(job.#get_args),*),
                        )
                })?;
            };
        }

        let get_ident = batch_get_entity_ident(&arg.id, &arg.over);
        let elem_cut_ub =
            arg.over
                .iter()
                .enumerate()
                .rfold(quote! { Ok(elem) }, |acc, (i, dim)| {
                    let dep_var = variable_ident(dim);
                    let res_map_ident = resolution_map_ident(dim);
                    let res_map_key = resolution_index
                        .get(dim)
                        .expect("Dimension not found in resolution index")
                        .mapped_over
                        .iter()
                        .map(|d| variable_ident(d));
                    let mut cnt = 0;

                    let mut msg_dims = Vec::new();
                    let mut msg_params: Vec<syn::Expr> = Vec::new();
                    for d in &entity.dims {
                        let var_ident = variable_ident(d);
                        if job_dim_set.contains(d) {
                            msg_dims.push(format!("{d} = {{}}"));
                            msg_params.push(parse_quote! { job.#var_ident });
                            continue;
                        }
                        if cnt < i {
                            msg_dims.push(format!("{d} = {{}}"));
                            msg_params.push(parse_quote! { #var_ident });
                        } else if cnt == i {
                            msg_dims.push(format!("{d} = *"))
                        } else {
                            msg_dims.push(format!("{d} = _"));
                        }
                        cnt += 1;
                    }
                    let not_found_msg = format!(
                        "{} ({}) expects {{}} elements, but only {{}} were found",
                        arg.id,
                        msg_dims.join(", ")
                    );
                    quote! {
                        let ub = #res_map_ident.get(&(#(#res_map_key,)*)).unwrap_or(&0); // TODO: Handle this better
                        if elem.len() < *ub {
                            return Err(#operon::storage::StorageError::NotFound(
                                format!(#not_found_msg, #(#msg_params,)* ub, elem.len())
                            ).into());
                        }
                        elem.into_iter()
                            .take(*ub)
                            .enumerate()
                            .map(|(#dep_var, elem)| { #acc })
                            .collect::<Result<Vec<_>, #operon::scheduler::SchedulerError>>()
                    }
                });

        parse_quote! {
            let #arg_ident = {
                let elem = storage.#get_ident(#(job.#get_args),*).await?;
                #elem_cut_ub
            }?;
        }
    });

    let job_fn_name = job_fn_ident(&job.id);

    let result_ident = entity_over_dim_ident(&job.to, job.spawn_dim.as_slice());
    let result_dims = job
        .dims
        .iter()
        .map(|dim| variable_ident(dim))
        .collect::<Vec<_>>();
    let put_fn_name = match &job.spawn_dim {
        Some(_) => batch_put_entity_ident(&job.to),
        None => put_entity_ident(&job.to),
    };
    let return_value: syn::Expr = match &job.spawn_dim {
        Some(dim) => {
            let res_ident = resolution_ident(dim);
            parse_quote! { schema::#res_ident(#result_ident.len(), #(job.#result_dims),*) }
        }
        None => parse_quote! { () },
    };

    parse_quote! {
        async fn run_job(
            &self,
            service: &Svc,
            storage: &Sto,
            client: #operon::meta_storage::MetaClient<'_>,
            job: &Self::Job,
        ) -> Result<Self::Resolution, #operon::scheduler::SchedulerError> {
            #(#resolution_defs)*
            #(#resolution_inserts)*

            #(#arg_defs)*

            let #result_ident = service
                .#job_fn_name(#(#args),*)
                .await
                .map_err(operon::scheduler::SchedulerError::UserError)?;
            let resolution = #return_value;

            storage.#put_fn_name(#(job.#result_dims,)* #result_ident).await?;
            Ok(resolution)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{EntityConfig, JobArg};

    use super::*;

    #[test]
    fn test_fn_run_job() {
        let entities = EntityConfigMap::from_iter([
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
        ]);
        let dimensions = DimensionConfigMap::from_iter([
            (
                "i".to_string(),
                DimensionConfig {
                    id: "i".to_string(),
                    depends_on: vec![],
                },
            ),
            (
                "j".to_string(),
                DimensionConfig {
                    id: "j".to_string(),
                    depends_on: vec!["i".to_string()],
                },
            ),
        ]);

        let job = JobConfig {
            id: "beta".to_string(),
            from: vec![JobArg {
                id: "a".to_string(),
                over: vec![],
            }],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
        };

        let item = fn_run_job(&job, &entities, &dimensions);
        let expected: syn::ImplItemFn = parse_quote! {
            async fn run_job(
                &self,
                service: &Svc,
                storage: &Sto,
                client: operon::meta_storage::MetaClient<'_>,
                job: &Self::Job,
            ) -> Result<Self::Resolution, operon::scheduler::SchedulerError> {
                let a = storage
                    .get_a(job.i)
                    .await?
                    .ok_or_else(|| {
                        operon::storage::StorageError::NotFound(
                            format!("a_{}", job.i),
                        )
                    })?;
                let b_j = service
                    .beta(a)
                    .await
                    .map_err(operon::scheduler::SchedulerError::UserError)?;

                let resolution = schema::JResolution(b_j.len(), job.i);
                storage.put_all_b(job.i, b_j).await?;

                Ok(resolution)
            }
        };

        assert_eq!(item, expected);
    }

    #[test]
    fn test_fn_run_with_over() {
        let entities = EntityConfigMap::from_iter([
            (
                "b".to_string(),
                EntityConfig {
                    id: "b".to_string(),
                    dims: vec!["i".to_string(), "j".to_string()],
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
        ]);
        let dimensions = DimensionConfigMap::from_iter([
            (
                "i".to_string(),
                DimensionConfig {
                    id: "i".to_string(),
                    depends_on: vec![],
                },
            ),
            (
                "j".to_string(),
                DimensionConfig {
                    id: "j".to_string(),
                    depends_on: vec!["i".to_string()],
                },
            ),
            (
                "k".to_string(),
                DimensionConfig {
                    id: "k".to_string(),
                    depends_on: vec!["i".to_string()],
                },
            ),
        ]);

        let job = JobConfig {
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
        };

        let item = fn_run_job(&job, &entities, &dimensions);
        let expected: syn::ImplItemFn = parse_quote! {
            async fn run_job(
                &self,
                service: &Svc,
                storage: &Sto,
                client: operon::meta_storage::MetaClient<'_>,
                job: &Self::Job,
            ) -> Result<Self::Resolution, operon::scheduler::SchedulerError> {
                let mut resolution_j: std::collections::HashMap<(), usize> = Default::default();
                let resolution = queries::get_resolution_j(client, job.i)
                    .await?
                    .ok_or_else(|| {
                        operon::meta_storage::MetaStorageError::MissingResolution(
                            format!("j_{}", job.i)
                        )
                    })?;
                resolution_j.insert((), resolution.0);

                let b_j = {
                    let elem = storage.get_all_b_over_j(job.i).await?;
                    let ub = resolution_j.get(&()).unwrap_or(&0);
                    if elem.len() < *ub {
                        return Err(operon::storage::StorageError::NotFound(
                            format!("b (i = {}, j = *) expects {} elements, but only {} were found", job.i, ub, elem.len()
                        )).into());
                    }
                    elem.into_iter()
                        .take(*ub)
                        .enumerate()
                        .map(|(j, elem)| { Ok(elem) })
                        .collect::<Result<Vec<_>, operon::scheduler::SchedulerError>>()
                }?;
                let d_j = {
                    let elem = storage.get_all_d_over_j(job.i, job.k).await?;
                    let ub = resolution_j.get(&()).unwrap_or(&0);
                    if elem.len() < *ub {
                        return Err(operon::storage::StorageError::NotFound(
                            format!("d (i = {}, j = *, k = {}) expects {} elements, but only {} were found", job.i, job.k, ub, elem.len())
                        ).into());
                    }
                    elem.into_iter()
                        .take(*ub)
                        .enumerate()
                        .map(|(j, elem)| { Ok(elem) })
                        .collect::<Result<Vec<_>, operon::scheduler::SchedulerError>>()
                }?;

                let e = service
                    .epsilon(b_j, d_j)
                    .await
                    .map_err(operon::scheduler::SchedulerError::UserError)?;
                let resolution = ();

                storage.put_e(job.i, job.k, e).await?;
                Ok(resolution)
            }
        };

        assert_eq!(item, expected);
    }

    #[test]
    fn test_fn_run_with_multi_dimensional_over() {
        let entities = EntityConfigMap::from_iter([
            (
                "b".to_string(),
                EntityConfig {
                    id: "b".to_string(),
                    dims: vec!["i".to_string(), "j".to_string()],
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
                    dims: vec!["i".to_string(), "l".to_string()],
                    body: String::new(),
                },
            ),
        ]);
        let dimensions = DimensionConfigMap::from_iter([
            (
                "i".to_string(),
                DimensionConfig {
                    id: "i".to_string(),
                    depends_on: vec![],
                },
            ),
            (
                "j".to_string(),
                DimensionConfig {
                    id: "j".to_string(),
                    depends_on: vec!["i".to_string()],
                },
            ),
            (
                "k".to_string(),
                DimensionConfig {
                    id: "k".to_string(),
                    depends_on: vec!["i".to_string(), "j".to_string()],
                },
            ),
            (
                "l".to_string(),
                DimensionConfig {
                    id: "l".to_string(),
                    depends_on: vec!["i".to_string()],
                },
            ),
        ]);

        let job = JobConfig {
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
        };

        let item = fn_run_job(&job, &entities, &dimensions);
        let expected: syn::ImplItemFn = parse_quote! {
            async fn run_job(
                &self,
                service: &Svc,
                storage: &Sto,
                client: operon::meta_storage::MetaClient<'_>,
                job: &Self::Job,
            ) -> Result<Self::Resolution, operon::scheduler::SchedulerError> {
                let mut resolution_j: std::collections::HashMap<(), usize> = Default::default();
                let mut resolution_k: std::collections::HashMap<(schema::JDim,), usize> = Default::default();

                let resolution = queries::get_resolution_j(client, job.i)
                    .await?
                    .ok_or_else(|| {
                        operon::meta_storage::MetaStorageError::MissingResolution(
                            format!("j_{}", job.i)
                        )
                    })?;
                resolution_j.insert((), resolution.0);

                for j in 0..(*resolution_j.get(&()).unwrap_or(&0)) { // TODO: Remove unwrap
                    let resolution = queries::get_resolution_k(client, job.i, j)
                    .await?
                        .ok_or_else(|| {
                            operon::meta_storage::MetaStorageError::MissingResolution(
                                format!("k_{},{}", job.i, j)
                            )
                        })?;
                    resolution_k.insert((j,), resolution.0);
                }

                let b_j = {
                    let elem = storage.get_all_b_over_j(job.i).await?;
                    let ub = resolution_j.get(&()).unwrap_or(&0);
                    if elem.len() < *ub {
                        return Err(operon::storage::StorageError::NotFound(
                            format!("b (i = {}, j = *) expects {} elements, but only {} were found", job.i, ub, elem.len())
                        ).into());
                    }
                    elem.into_iter()
                        .take(*ub)
                        .enumerate()
                        .map(|(j, elem)| { Ok(elem) })
                        .collect::<Result<Vec<_>, operon::scheduler::SchedulerError>>()
                }?;
                let d_j_k = {
                    let elem = storage.get_all_d_over_jk(job.i).await?;
                    let ub = resolution_j.get(&()).unwrap_or(&0);
                    if elem.len() < *ub {
                        return Err(operon::storage::StorageError::NotFound(
                            format!("d (i = {}, j = *, k = _) expects {} elements, but only {} were found", job.i, ub, elem.len())
                        ).into());
                    }
                    elem.into_iter()
                        .take(*ub)
                        .enumerate()
                        .map(|(j, elem)| {
                            let ub = resolution_k.get(&(j,)).unwrap_or(&0);
                            if elem.len() < *ub {
                                return Err(operon::storage::StorageError::NotFound(
                                    format!("d (i = {}, j = {}, k = *) expects {} elements, but only {} were found", job.i, j, ub, elem.len())
                                ).into());
                            }
                            elem.into_iter()
                                .take(*ub)
                                .enumerate()
                                .map(|(k, elem)| { Ok(elem) })
                                .collect::<Result<Vec<_>, operon::scheduler::SchedulerError>>()
                        })
                        .collect::<Result<Vec<_>, operon::scheduler::SchedulerError>>()
                }?;

                let e_l = service
                    .epsilon(b_j, d_j_k)
                    .await
                    .map_err(operon::scheduler::SchedulerError::UserError)?;
                let resolution = schema::LResolution(e_l.len(), job.i);

                storage.put_all_e(job.i, e_l).await?;
                Ok(resolution)
            }
        };

        assert_eq!(item, expected);
    }
}
