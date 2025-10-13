use heck::ToSnakeCase;
use indexmap::{IndexMap, IndexSet};
use quote::format_ident;
use syn::parse_quote;

use crate::configs::{DimensionConfigMap, DimensionId, EntityId};
use crate::utils::{dimension_ident, get_resolution_ident, operon_ident, variable_ident};
use crate::{DimensionConfig, JobConfig, JobConfigMap};

fn resolution_map_ident(dim: &DimensionId) -> syn::Ident {
    format_ident!("resolution_{}", dim.to_snake_case())
}

struct CalculatedDependency<'a> {
    single_job_cnt: usize,
    required_dims_from_self: IndexSet<&'a DimensionId>,
    resolution_index: IndexMap<&'a DimensionId, ResolutionIndexEntry<'a>>,
}

struct ResolutionIndexEntry<'a> {
    config: &'a DimensionConfig,
    fetched_over: Vec<&'a DimensionId>,
    count: usize,
}

/// Builds an index map that contains the resolution entries which needs to be fetched from a
/// metadata storage which stores required information for the generation of the `run_job` function.
///
/// The index map is keyed by the dimension ID and contains entries that specify the dimension
/// configuration and the dependencies that need to be collected over.
fn calculate_dependency<'a>(
    job: &'a JobConfig,
    primary_entity: &'a EntityId,
    jobs: &'a JobConfigMap,
    dimensions: &'a DimensionConfigMap,
) -> CalculatedDependency<'a> {
    let mut single_job_cnt = 0usize;
    let mut required_dims_from_ticket = IndexSet::new();
    let mut resolution_index: IndexMap<&DimensionId, ResolutionIndexEntry<'_>> = IndexMap::new();

    let job_dim_set = job.dims.iter().collect::<IndexSet<_>>();

    for arg in &job.from {
        if arg.id == *primary_entity {
            continue; // Skip the primary entity as it should always be resolved
        }

        let arg_job_dims = jobs
            .values()
            .find(|j| j.to == arg.id)
            .unwrap_or_else(|| panic!("Job generating `{}` not found", arg.id))
            .dims
            .iter()
            .collect::<IndexSet<_>>();

        let unincluded_dims = arg_job_dims
            .difference(&job_dim_set)
            .cloned()
            .collect::<Vec<_>>();

        if unincluded_dims.is_empty() {
            single_job_cnt += 1;
            continue;
        }

        for unincluded_dim in arg_job_dims.difference(&job_dim_set).cloned() {
            if let Some(entry) = resolution_index.get_mut(unincluded_dim) {
                entry.count += 1;
                continue;
            }

            let config = dimensions.get(unincluded_dim).unwrap_or_else(|| {
                panic!("Dimension `{unincluded_dim}` not found in config");
            });
            let (from_ticket, fetched_over) = config
                .depends_on
                .iter()
                .partition(|dep_dim| job_dim_set.contains(dep_dim));
            required_dims_from_ticket.extend(from_ticket);

            let entry = ResolutionIndexEntry {
                config,
                fetched_over,
                count: 1,
            };
            resolution_index.insert(unincluded_dim, entry);
        }
    }

    CalculatedDependency {
        single_job_cnt,
        required_dims_from_self: required_dims_from_ticket,
        resolution_index,
    }
}

/// Generates the code that fetches and inserts the resolution entries into the resolution map.
fn resolution_inserts(
    resolution_index: &IndexMap<&DimensionId, ResolutionIndexEntry>,
) -> impl Iterator<Item = syn::Expr> {
    resolution_index.iter().map(|(dim, entry)| {
        let res_map = resolution_map_ident(dim);
        let get_resolution_fn_name = get_resolution_ident(dim);
        let get_resolution_args = entry
            .config
            .depends_on
            .iter()
            .map(|dep| variable_ident(dep))
            .collect::<Vec<_>>();
        let dep_vars = entry.fetched_over.iter().map(|dep| variable_ident(dep));

        entry.fetched_over.iter().rfold(
            parse_quote! {
                if let Some(resolution) = queries::#get_resolution_fn_name(client, #(#get_resolution_args),*).await? {
                    #res_map.insert((#(#dep_vars,)*), resolution.0);
                } else {
                    return Ok(None);
                }
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

                parse_quote! {
                    for #dep_var in 0..(*#dep_res_map.get(&(#(#dep_ub_key,)*)).unwrap_or(&0)) { // TODO: Remove unwrap
                        #acc
                    }
                }
            },
        )
    })
}

pub(super) fn fn_get_dependency_quota(
    job: &JobConfig,
    primary_entity: &EntityId,
    dimensions: &DimensionConfigMap,
    jobs: &JobConfigMap,
) -> syn::ImplItemFn {
    let operon = operon_ident();

    let CalculatedDependency {
        single_job_cnt,
        required_dims_from_self,
        resolution_index,
    } = calculate_dependency(job, primary_entity, jobs, dimensions);

    let from_self_defs: Vec<syn::Stmt> = required_dims_from_self
        .iter()
        .map(|dim| {
            let dim_ident = variable_ident(dim);
            parse_quote! {
                let Some(#dim_ident) = self.#dim_ident.0 else {
                    return Ok(None);
                };
            }
        })
        .collect::<Vec<_>>();

    let resolution_defs = resolution_index.iter().map(|(dim, entry)| -> syn::Stmt {
        let res_map_var = resolution_map_ident(dim);
        let key_ty = entry.fetched_over.iter().map(|dep| dimension_ident(dep));
        let ty: syn::Type = parse_quote! {
            std::collections::HashMap<(#(schema::#key_ty,)*), usize>
        };
        parse_quote! { let mut #res_map_var: #ty = Default::default(); }
    });
    let resolution_inserts = resolution_inserts(&resolution_index);

    let resolution_count = resolution_index.iter().map(|(dim, entry)| -> syn::Expr {
        let resolution_map = resolution_map_ident(dim);
        let count = entry.count;
        parse_quote! { #resolution_map.values().sum::<usize>() * #count }
    });

    parse_quote! {
        #[allow(unused_variables)]
        async fn get_dependency_quota(
            &self,
            client: #operon::meta_storage::MetaClient<'_>,
        ) -> Result<Option<usize>, #operon::meta_storage::MetaStorageError> {
            #(#from_self_defs)*
            #(#resolution_defs)*
            #(#resolution_inserts)*

            Ok(Some(#single_job_cnt #(+ #resolution_count)*))
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_dimensions, all_jobs, primary_entity};

    #[rstest]
    #[case::empty("beta", "schema/ticket/fn_get_dependency_quota.empty.rs")]
    #[case::with_dependency("epsilon", "schema/ticket/fn_get_dependency_quota.with_dependency.rs")]
    fn test_fn_get_dependency_quota(
        primary_entity: EntityId,
        all_dimensions: DimensionConfigMap,
        all_jobs: JobConfigMap,
        #[case] job_id: &str,
        #[case] fixture_path: &str,
    ) {
        let job = all_jobs.get(job_id).unwrap();
        let item = fn_get_dependency_quota(job, &primary_entity, &all_dimensions, &all_jobs);
        assert_item_eq(&item, fixture_path);
    }
}
