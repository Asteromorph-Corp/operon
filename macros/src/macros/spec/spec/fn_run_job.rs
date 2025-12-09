use heck::ToSnakeCase;
use indexmap::{IndexMap, IndexSet};
use quote::{format_ident, quote};
use syn::parse_quote;

use crate::configs::{
    DimensionConfig, DimensionConfigMap, EntityConfig, EntityConfigMap, JobConfig,
};
use crate::utils::{
    batch_get_entity_ident, batch_put_entity_ident, clear_span, dim_msg, dimension_metadata_ident,
    entity_over_dim_ident, get_entity_ident, operon_ident, put_entity_ident,
};

fn resolution_map_ident(dim: &syn::Ident) -> syn::Ident {
    format_ident!("resolution_{}", dim.to_string().to_snake_case())
}

struct ResolutionIndexEntry<'a> {
    config: &'a DimensionConfig,
    /// The dimensions that this resolution is fetched over.
    ///
    /// Note that this is a subset of, but not identical to `config.depends_on`,
    /// as dimensions not included here are already part of the job dimensions
    /// so we need to only fetch the resolution for the specific value of that dimension.
    fetched_over: Vec<&'a syn::Ident>,
}

/// Builds an index map that contains the resolution entries which needs to be fetched from a
/// metadata storage which stores required information for the generation of the `run_job` function.
///
/// The index map is keyed by the dimension ID and contains entries that specify the dimension
/// configuration and the dependencies that need to be collected over.
fn build_resolution_index<'a>(
    job: &'a JobConfig,
    job_dim_set: &'a IndexSet<&'a syn::Ident>,
    dimensions: &'a DimensionConfigMap,
) -> IndexMap<&'a syn::Ident, ResolutionIndexEntry<'a>> {
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
    resolution_index: &IndexMap<&syn::Ident, ResolutionIndexEntry>,
) -> impl Iterator<Item = proc_macro2::TokenStream> {
    resolution_index.iter().map(|(&dim, entry)| {
        let operon = operon_ident();
        let dim_meta = dimension_metadata_ident(dim);
        let res_map = resolution_map_ident(dim);

        let get_args = entry
            .config
            .depends_on
            .iter()
            .map(clear_span);
        let dep_vars = entry.fetched_over.iter().cloned().map(clear_span);

        let dim_msgs = entry.config.depends_on.iter().map(dim_msg).collect::<Vec<_>>();
        let not_found_msg = format!("{} ({})", dim, dim_msgs.join(", "));

        entry.fetched_over.iter().rfold(
            quote! {
                let Some(resolution) = client.resolution(metadata::#dim_meta()).get([#(#get_args),*]).await? else {
                    return Err(#operon::meta_storage::MetaStorageError::MissingResolution(
                        format!(#not_found_msg)
                    ).into());
                };
                #res_map.insert([#(#dep_vars),*], resolution.ub);
            },
            |acc, dep| {
                let Some(dep_config) = resolution_index.get(dep) else {
                    panic!("Dimension `{dep}` not found in resolution index");
                };
                let dep_res_map = resolution_map_ident(dep);
                let dep = clear_span(dep);
                let dep_ub_key = dep_config.fetched_over.iter().cloned().map(clear_span);

                quote! {
                    for #dep in 0..(*#dep_res_map.get(&[#(#dep_ub_key),*]).unwrap_or(&0)) { // TODO: Remove unwrap
                        #acc
                    }
                }
            },
        )
    })
}

/// Generates the code for defining a single argument entity in the `run_job` function.
fn arg_def_single(arg_entity: &EntityConfig) -> syn::Stmt {
    let operon = operon_ident();
    let arg_ident = entity_over_dim_ident(&arg_entity.id, &[]);
    let get_ident = get_entity_ident(&arg_entity.id);

    // All arg dimension are passed to the get function
    // These dimensions are expected to be present in the job struct
    let get_args = arg_entity.dims.iter().map(clear_span);

    let dim_msgs = arg_entity.dims.iter().map(dim_msg).collect::<Vec<_>>();
    let not_found_msg = format!("{} ({})", arg_entity.id, dim_msgs.join(", "));

    parse_quote! {
        let Some(#arg_ident) = storage
            .#get_ident([#(#get_args),*])
            .await?
        else {
            return Err(#operon::storage::StorageError::NotFound(format!(#not_found_msg)).into());
        };
    }
}

/// Generates the code for defining a collected argument entity in the `run_job` function.
fn arg_def_collected(
    arg_entity: &EntityConfig,
    over: &[syn::Ident],
    resolution_index: &IndexMap<&syn::Ident, ResolutionIndexEntry>,
    job_dim_set: &IndexSet<&syn::Ident>,
) -> syn::Stmt {
    let operon = operon_ident();
    let arg_ident = entity_over_dim_ident(&arg_entity.id, over);
    let get_ident = batch_get_entity_ident(&arg_entity.id, over);

    // Only arg dimensions that are not part of the `over` dimensions are passed to the get function
    // These dimensions are expected to be present in the job struct
    let get_args = arg_entity
        .dims
        .iter()
        .filter(|d| !over.contains(d))
        .map(clear_span)
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
            let dim = clear_span(dim);

            let res_map_ident = resolution_map_ident(&dim);
            let res_map_key = resolution_index
                .get(&dim)
                .expect("Dimension not found in resolution index")
                .fetched_over
                .iter();

            let mut cnt = 0;
            let dim_msgs = arg_entity
                .dims
                .iter()
                .map(|arg_dim| {
                    if job_dim_set.contains(arg_dim) {
                        return dim_msg(arg_dim);
                    }
                    cnt += 1;
                    if cnt <= i {
                        dim_msg(arg_dim)
                    } else if cnt == i + 1 {
                        format!("{arg_dim} = *")
                    } else {
                        format!("{arg_dim} = _")
                    }
                })
                .collect::<Vec<_>>();
            let not_found_msg = format!(
                "{} ({}) expects {{ub}} elements, but only {{len}} were found",
                arg_entity.id,
                dim_msgs.join(", ")
            );

            quote! {
                let len = elem.len();
                let ub = #res_map_ident.get(&[#(#res_map_key,)*]).unwrap_or(&0); // TODO: Handle this better
                if len < *ub {
                    return Err(#operon::storage::StorageError::NotFound(
                        format!(#not_found_msg)
                    ).into());
                }
                elem.into_iter()
                    .take(*ub)
                    .enumerate()
                    .map(|(#dim, elem)| #acc)
                    .collect::<Result<Vec<_>, #operon::scheduler::SchedulerError>>()
            }
        });

    parse_quote! {
        let #arg_ident = {
            let elem = storage.#get_ident([#(#get_args),*]).await?;
            #check_ub
        }?;
    }
}

/// Generates the `run_job` function for the implementation of the trait `JobSpec`.
///
/// # Example
/// ```rust,ignore
/// async fn run_job(
///     &self,
///     service: &Svc,
///     storage: &Sto,
///     client: operon::meta_storage::MetaClient<'_>,
///     job: Self::Job,
/// ) -> Result<Self::Resolution, operon::scheduler::SchedulerError> {
///     let [i] = job.coordinate;
///
///     let Some(a) = storage.get_a([i]).await? else {
///         return Err(operon::storage::StorageError::NotFound(format!("a (i = {i})")).into());
///     };
///
///     let b_j = service
///         .beta(a)
///         .await
///         .map_err(operon::scheduler::SchedulerError::UserError)?;
///     let entity = operon::schema::Entity {
///         coordinate: job.coordinate,
///         value: b_j,
///     };
///     let resolution = operon::schema::Resolution::new(entity.value.len(), job.coordinate);
///
///     storage.put_all_b(entity).await?;
///     client
///         .resolution(self.spawn_dim_meta())
///         .put(resolution)
///         .await?;
///     Ok(resolution)
/// }
/// ```
pub(super) fn fn_run_job(
    job: &JobConfig,
    entities: &EntityConfigMap,
    dimensions: &DimensionConfigMap,
) -> syn::ImplItemFn {
    let operon = operon_ident();

    let job_dim_set: IndexSet<&syn::Ident> = job.dims.iter().collect();
    let resolution_index = build_resolution_index(job, &job_dim_set, dimensions);

    let job_coord_vars = job_dim_set.iter().cloned().map(clear_span);
    let resolution_defs = resolution_index.iter().map(|(dim, entry)| -> syn::Stmt {
        let res_map_var = resolution_map_ident(dim);
        let n = entry.fetched_over.len();
        let ty: syn::Type = parse_quote! {
            std::collections::HashMap<[usize; #n], usize>
        };
        parse_quote! { let mut #res_map_var: #ty = Default::default(); }
    });
    let resolution_inserts = resolution_inserts(&resolution_index);

    let job_fn_name = clear_span(&job.id);
    let args = job
        .from
        .iter()
        .map(|arg| entity_over_dim_ident(&arg.id, &arg.over));
    let arg_defs = job.from.iter().map(|arg| -> syn::Stmt {
        let entity = entities.get(&arg.id).expect("Entity not found in config");
        if arg.over.is_empty() {
            arg_def_single(entity)
        } else {
            arg_def_collected(entity, &arg.over, &resolution_index, &job_dim_set)
        }
    });

    let result_ident = entity_over_dim_ident(&job.to, job.spawn_dim.as_slice());
    let put_fn_name = match &job.spawn_dim {
        Some(_) => batch_put_entity_ident(&job.to),
        None => put_entity_ident(&job.to),
    };

    let entity: syn::Expr = parse_quote! {
        #operon::schema::Entity {
            coordinate: job.coordinate,
            value: #result_ident,
        }
    };
    let resolution: syn::Expr = if job.spawn_dim.is_some() {
        parse_quote! { #operon::schema::Resolution::new(entity.value.len(), job.coordinate)  }
    } else {
        parse_quote! { () }
    };

    let maybe_put_resolution: Option<syn::Stmt> = job.spawn_dim.is_some().then(|| {
        parse_quote! {
            client.resolution(self.spawn_dim_meta()).put(resolution).await?;
        }
    });

    parse_quote! {
        #[allow(unused_variables)]
        async fn run_job(
            &self,
            service: &Svc,
            storage: &Sto,
            client: #operon::meta_storage::MetaClient<'_>,
            job: Self::Job,
        ) -> Result<Self::Resolution, #operon::scheduler::SchedulerError> {
            let [#(#job_coord_vars),*] = job.coordinate;
            #(#resolution_defs)*
            #(#resolution_inserts)*

            #(#arg_defs)*

            let #result_ident = service
                .#job_fn_name(#(#args),*)
                .await
                .map_err(operon::scheduler::SchedulerError::UserError)?;
            let entity = #entity;
            let resolution = #resolution;

            storage.#put_fn_name(entity).await?;
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
