use heck::ToSnakeCase;
use indexmap::{IndexMap, IndexSet};
use quote::{format_ident, quote};
use syn::parse_quote;

use crate::configs::{
    DimensionConfig, DimensionConfigMap, DimensionId, EntityConfig, EntityConfigMap, JobConfig,
};
use crate::utils::{
    batch_get_entity_ident, batch_put_entity_ident, dimension_metadata_ident,
    entity_over_dim_ident, get_entity_ident, job_fn_ident, operon_ident, put_entity_ident,
    variable_ident,
};

fn resolution_map_ident(dim: &DimensionId) -> syn::Ident {
    format_ident!("resolution_{}", dim.to_snake_case())
}

struct ResolutionIndexEntry<'a> {
    config: &'a DimensionConfig,
    /// The dimensions that this resolution is fetched over.
    ///
    /// Note that this is a subset of, but not identical to `config.depends_on`,
    /// as dimensions not included here are already part of the job dimensions
    /// so we need to only fetch the resolution for the specific value of that dimension.
    fetched_over: Vec<&'a DimensionId>,
}

/// Builds an index map that contains the resolution entries which needs to be fetched from a
/// metadata storage which stores required information for the generation of the `run_job` function.
///
/// The index map is keyed by the dimension ID and contains entries that specify the dimension
/// configuration and the dependencies that need to be collected over.
fn build_resolution_index<'a>(
    job: &'a JobConfig,
    job_dim_set: &'a IndexSet<&'a DimensionId>,
    dimensions: &'a DimensionConfigMap,
) -> IndexMap<&'a DimensionId, ResolutionIndexEntry<'a>> {
    job.from
        .iter()
        .flat_map(|arg| {
            arg.over.iter().map(|dim| {
                let config = dimensions.get(dim).unwrap_or_else(|| {
                    panic!("Dimension `{dim}` not found in config");
                });

                // Collect dependencies that are not part of the job dimensions
                // In the generated code, the tuple of these dimensions are used as keys in the
                // resolution map
                let fetched_over = config
                    .depends_on
                    .iter()
                    .filter(|dep_dim| !job_dim_set.contains(dep_dim))
                    .collect::<Vec<_>>();

                let entry = ResolutionIndexEntry {
                    config,
                    fetched_over,
                };

                (dim, entry)
            })
        })
        .collect()
}

/// Generates the code that fetches and inserts the resolution entries into the resolution map.
fn resolution_inserts(
    resolution_index: &IndexMap<&DimensionId, ResolutionIndexEntry>,
    job_dim_set: &IndexSet<&DimensionId>,
) -> impl Iterator<Item = proc_macro2::TokenStream> {
    resolution_index.iter().map(|(&dim, entry)| {
        let operon = operon_ident();
        let dim_meta = dimension_metadata_ident(dim);
        let res_map = resolution_map_ident(dim);
        let get_resolution_args = entry
            .config
            .depends_on
            .iter()
            .map(|dep| -> syn::Expr {
                if let Some(index) = job_dim_set.iter().position(|d| *d == dep) {
                    parse_quote! { job.coordinate[#index] }
                } else {
                    let dep_var = variable_ident(dep);
                    parse_quote! { #dep_var }
                }
            })
            .collect::<Vec<_>>();
        let dep_vars = entry.fetched_over.iter().map(|dep| variable_ident(dep));

        let missing_msg = if entry.config.depends_on.is_empty() {
            dim.clone()
        } else {
            format!("{dim}_{{:?}}")
        };

        entry.fetched_over.iter().rfold(
            quote! {
                let pkey = [#(#get_resolution_args),*];
                let Some(resolution) = client.resolution(metadata::#dim_meta()).get(pkey).await? else {
                    return Err(#operon::meta_storage::MetaStorageError::MissingResolution(
                        format!(#missing_msg, pkey)
                    ).into());
                };
                #res_map.insert([#(#dep_vars),*], resolution.ub);
            },
            |acc, dep| {
                let dep_var = variable_ident(dep);
                let dep_res_map = resolution_map_ident(dep);
                let dep_ub_key = resolution_index
                    .get(dep)
                    .unwrap_or_else(|| {
                        panic!("Dimension `{dep}` not found in resolution index");
                    })
                    .fetched_over
                    .iter()
                    .map(|d| variable_ident(d));

                quote! {
                    for #dep_var in 0..(*#dep_res_map.get(&[#(#dep_ub_key),*]).unwrap_or(&0)) { // TODO: Remove unwrap
                        #acc
                    }
                }
            },
        )
    })
}

/// Generates the code for defining a single argument entity in the `run_job` function.
fn arg_def_single(job: &JobConfig, arg_entity: &EntityConfig) -> syn::Stmt {
    let operon = operon_ident();
    let arg_ident = entity_over_dim_ident(&arg_entity.id, &[]);
    let get_ident = get_entity_ident(&arg_entity.id);

    // All arg dimension are passed to the get function
    // These dimensions are expected to be present in the job struct
    let get_indices = arg_entity
        .dims
        .iter()
        .map(|arg_dim| {
            job.dims
                .iter()
                .position(|d| d == arg_dim)
                .unwrap_or_else(|| panic!("Missing dimension {} in job {}", arg_dim, job.id))
        })
        .collect::<Vec<_>>();

    let not_found_msg = format!(
        "{}_{}",
        arg_entity.id,
        "{},".repeat(arg_entity.dims.len()).trim_end_matches(",")
    );

    parse_quote! {
        let Some(#arg_ident) = storage
            .#get_ident(#(job.coordinate[#get_indices]),*)
            .await?
        else {
            return Err(#operon::storage::StorageError::NotFound(format!(#not_found_msg, #(job.coordinate[#get_indices]),*)).into());
        };
    }
}

/// Generates the code for defining a collected argument entity in the `run_job` function.
fn arg_def_collected(
    job: &JobConfig,
    arg_entity: &EntityConfig,
    over: &[DimensionId],
    resolution_index: &IndexMap<&DimensionId, ResolutionIndexEntry>,
    job_dim_set: &IndexSet<&DimensionId>,
) -> syn::Stmt {
    let operon = operon_ident();
    let arg_ident = entity_over_dim_ident(&arg_entity.id, over);
    let get_ident = batch_get_entity_ident(&arg_entity.id, over);

    // Only arg dimensions that are not part of the `over` dimensions are passed to the get function
    // These dimensions are expected to be present in the job struct
    let get_indices = arg_entity
        .dims
        .iter()
        .filter(|dim| !over.contains(dim))
        .map(|arg_dim| {
            job.dims
                .iter()
                .position(|d| d == arg_dim)
                .unwrap_or_else(|| panic!("Missing dimension {} in job {}", arg_dim, job.id))
        })
        .collect::<Vec<_>>();

    let check_ub = over
        .iter()
        .enumerate()
        .rfold(quote! { Ok(elem) }, |acc, (i, dim)| {
            let acc = if i == over.len() - 1 {
                acc
            } else {
                quote! { { #acc } }
            };

            let dep_var = variable_ident(dim);
            let res_map_ident = resolution_map_ident(dim);
            let res_map_key = resolution_index
                .get(dim)
                .expect("Dimension not found in resolution index")
                .fetched_over
                .iter()
                .map(|d| variable_ident(d));

            let mut cnt = 0;
            let (msg_dims, msg_params): (Vec<_>, Vec<_>) = arg_entity
                .dims
                .iter()
                .map(|arg_dim| {
                    if let Some(index) = job_dim_set.iter().position(|d| *d == arg_dim) {
                        return (
                            format!("{arg_dim} = {{}}"),
                            Some(parse_quote! { job.coordinate[#index] }),
                        );
                    }
                    cnt += 1;
                    if cnt <= i {
                        let var_ident = variable_ident(arg_dim);
                        (
                            format!("{arg_dim} = {{}}"),
                            Some(parse_quote! { #var_ident }),
                        )
                    } else if cnt == i + 1 {
                        (format!("{arg_dim} = *"), None)
                    } else {
                        (format!("{arg_dim} = _"), None)
                    }
                })
                .unzip();
            let not_found_msg = format!(
                "{} ({}) expects {{}} elements, but only {{}} were found",
                arg_entity.id,
                msg_dims.join(", ")
            );
            let msg_params: Vec<syn::Expr> = msg_params.into_iter().flatten().collect();

            quote! {
                let ub = #res_map_ident.get(&[#(#res_map_key,)*]).unwrap_or(&0); // TODO: Handle this better
                if elem.len() < *ub {
                    return Err(#operon::storage::StorageError::NotFound(
                        format!(#not_found_msg, #(#msg_params,)* ub, elem.len())
                    ).into());
                }
                elem.into_iter()
                    .take(*ub)
                    .enumerate()
                    .map(|(#dep_var, elem)| #acc)
                    .collect::<Result<Vec<_>, #operon::scheduler::SchedulerError>>()
            }
        });

    parse_quote! {
        let #arg_ident = {
            let elem = storage.#get_ident(#(job.coordinate[#get_indices]),*).await?;
            #check_ub
        }?;
    }
}

/// Generates the `run_job` function for the implementation of the trait `JobSpec`.
///
/// Example:
/// ```rust,ignore
/// async fn run_job(
///     &self,
///     service: &Svc,
///     storage: &Sto,
///     client: operon::meta_storage::MetaClient<'_>,
///     job: &Self::Job,
/// ) -> Result<Self::Resolution, operon::scheduler::SchedulerError> {
///     let mut resolution_j: std::collections::HashMap<(), usize> = Default::default();
///     let resolution = queries::get_resolution_j(client, job.i)
///         .await?
///         .ok_or_else(|| {
///             operon::meta_storage::MetaStorageError::MissingResolution(
///                 format!("j_{}", job.i)
///             )
///         })?;
///     resolution_j.insert((), resolution.0);
///
///     let b_j = {
///         let elem = storage.get_all_b_over_j(job.i).await?;
///         let ub = resolution_j.get(&()).unwrap_or(&0);
///         if elem.len() < *ub {
///             return Err(operon::storage::StorageError::NotFound(
///                 format!("b (i = {}, j = *) expects {} elements, but only {} were found", job.i, ub, elem.len()
///             )).into());
///         }
///         elem.into_iter()
///             .take(*ub)
///             .enumerate()
///             .map(|(j, elem)| Ok(elem))
///             .collect::<Result<Vec<_>, operon::scheduler::SchedulerError>>()
///     }?;
///     let d_j = {
///         let elem = storage.get_all_d_over_j(job.i, job.k).await?;
///         let ub = resolution_j.get(&()).unwrap_or(&0);
///         if elem.len() < *ub {
///             return Err(operon::storage::StorageError::NotFound(
///                 format!("d (i = {}, j = *, k = {}) expects {} elements, but only {} were found", job.i, job.k, ub, elem.len())
///             ).into());
///         }
///         elem.into_iter()
///             .take(*ub)
///             .enumerate()
///             .map(|(j, elem)| Ok(elem))
///             .collect::<Result<Vec<_>, operon::scheduler::SchedulerError>>()
///     }?;
///
///     let e = service
///         .epsilon(b_j, d_j)
///         .await
///         .map_err(operon::scheduler::SchedulerError::UserError)?;
///     let resolution = ();
///
///     storage.put_e(job.i, job.k, e).await?;
///     Ok(resolution)
/// }
/// ```
pub(super) fn fn_run_job(
    job: &JobConfig,
    entities: &EntityConfigMap,
    dimensions: &DimensionConfigMap,
) -> syn::ImplItemFn {
    let operon = operon_ident();

    let job_dim_set: IndexSet<&DimensionId> = job.dims.iter().collect();
    let resolution_index = build_resolution_index(job, &job_dim_set, dimensions);

    let resolution_defs = resolution_index.iter().map(|(dim, entry)| -> syn::Stmt {
        let res_map_var = resolution_map_ident(dim);
        let n = entry.fetched_over.len();
        let ty: syn::Type = parse_quote! {
            std::collections::HashMap<[usize; #n], usize>
        };
        parse_quote! { let mut #res_map_var: #ty = Default::default(); }
    });
    let resolution_inserts = resolution_inserts(&resolution_index, &job_dim_set);

    let job_fn_name = job_fn_ident(&job.id);
    let args = job
        .from
        .iter()
        .map(|arg| entity_over_dim_ident(&arg.id, &arg.over));
    let arg_defs = job.from.iter().map(|arg| -> syn::Stmt {
        let entity = entities.get(&arg.id).expect("Entity not found in config");
        if arg.over.is_empty() {
            arg_def_single(job, entity)
        } else {
            arg_def_collected(job, entity, &arg.over, &resolution_index, &job_dim_set)
        }
    });

    let result_ident = entity_over_dim_ident(&job.to, job.spawn_dim.as_slice());
    let put_fn_name = match &job.spawn_dim {
        Some(_) => batch_put_entity_ident(&job.to),
        None => put_entity_ident(&job.to),
    };

    let resolution: syn::Expr = if job.spawn_dim.is_some() {
        parse_quote! { #operon::schema_base::Resolution::new(#result_ident.len(), job.coordinate)  }
    } else {
        parse_quote! { () }
    };
    let maybe_put_resolution: Option<syn::Stmt> = job.spawn_dim.is_some().then(|| {
        parse_quote! {
            client.resolution(self.spawn_dim_meta()).put(resolution).await?;
        }
    });

    let indices = 0..job.dims.len();

    parse_quote! {
        async fn run_job(
            &self,
            service: &Svc,
            storage: &Sto,
            client: #operon::meta_storage::MetaClient<'_>,
            job: Self::Job,
        ) -> Result<Self::Resolution, #operon::scheduler::SchedulerError> {
            #(#resolution_defs)*
            #(#resolution_inserts)*

            #(#arg_defs)*

            let #result_ident = service
                .#job_fn_name(#(#args),*)
                .await
                .map_err(operon::scheduler::SchedulerError::UserError)?;
            let resolution = #resolution;

            storage.#put_fn_name(#(job.coordinate[#indices],)* #result_ident).await?;
            #maybe_put_resolution;
            Ok(resolution)
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::complicated_pipeline::{
        all_dimensions as all_dimensions_complicated, all_entities as all_entities_complicated,
        job_epsilon as job_multiple_over,
    };
    use crate::test_utils::simple_pipeline::{all_dimensions, all_entities, job_beta, job_epsilon};

    #[rstest]
    #[case::simple(
        all_entities(),
        all_dimensions(),
        job_beta(),
        "spec/spec/fn_run_job.simple.rs"
    )]
    #[case::with_over(
        all_entities(),
        all_dimensions(),
        job_epsilon(),
        "spec/spec/fn_run_job.with_over.rs"
    )]
    #[case::multiple_over(
        all_entities_complicated(),
        all_dimensions_complicated(),
        job_multiple_over(),
        "spec/spec/fn_run_job.multiple_over.rs"
    )]
    fn test_fn_run_job(
        #[case] all_entities: EntityConfigMap,
        #[case] all_dimensions: DimensionConfigMap,
        #[case] job: JobConfig,
        #[case] fixture_path: &str,
    ) {
        let item = fn_run_job(&job, &all_entities, &all_dimensions);
        assert_item_eq(&item, fixture_path);
    }
}
