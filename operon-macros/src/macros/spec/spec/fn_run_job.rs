use std::borrow::Borrow;

use heck::ToSnakeCase;
use indexmap::IndexSet;
use quote::{format_ident, quote};
use syn::parse_quote;

use crate::configs::{
    DimensionConfig, DimensionConfigMap, EntityConfig, EntityConfigMap, JobArg, JobConfig,
};
use crate::utils::{
    batch_get_entity_ident, batch_put_entity_ident, clear_span, dimension_metadata_ident,
    entity_over_dim_ident, get_entity_ident, operon_ident, put_entity_ident,
};

fn resolution_map_ident(dim: &RequiredDim<'_>) -> syn::Ident {
    let id = dim.config.id.to_string().to_snake_case();
    if dim.fetched_over.is_empty() {
        return format_ident!("resolution_{id}");
    }

    let over = dim
        .fetched_over
        .iter()
        .map(|d| d.to_string().to_snake_case())
        .collect::<String>();
    format_ident!("resolution_{id}_{over}")
}

/// Represents a dimension whose resolution need to be fetched in order to check if all entities are
/// present.
#[derive(Hash, PartialEq, Eq)]
struct RequiredDim<'a> {
    config: &'a DimensionConfig,
    /// The dimensions that this resolution is fetched over.
    ///
    /// Note that this is a subset of, but not identical to `config.depends_on`,
    /// as dimensions not included here are already part of the job dimensions
    /// so we need to only fetch the resolution for the specific value of that dimension.
    fetched_over: Vec<&'a syn::Ident>,
}

impl<'a> RequiredDim<'a> {
    /// Constructs a new [`RequiredDim`] object with a aggregation dimension filter.
    fn new(config: &'a DimensionConfig, over: &[impl Borrow<syn::Ident>]) -> Self {
        let fetched_over = config
            .depends_on
            .iter()
            .filter(|dep_dim| over.iter().any(|d| d.borrow() == *dep_dim))
            .collect::<Vec<_>>();
        Self {
            config,
            fetched_over,
        }
    }

    /// Gets the "parent" [`RequiredDim`] configuration for one of the dependant dimension.
    fn parent(&self, dim: &'a DimensionConfig) -> Self {
        // `self.fetched_over` is a subset of the original aggregation dimension that `self`
        // depended on, so it can be used as the aggregation dimension.
        Self::new(dim, &self.fetched_over)
    }
}

/// Returns dimensions whose resolution need to be fetched in order to check entity vector length
/// for an argument.
fn get_required_dims<'a>(
    arg: &'a JobArg,
    dimensions: &'a DimensionConfigMap,
) -> impl Iterator<Item = RequiredDim<'a>> {
    arg.over.iter().map(|dim| {
        let config = dimensions.get(dim).unwrap_or_else(|| {
            panic!("Dimension `{dim}` not found in config");
        });
        RequiredDim::new(config, &arg.over)
    })
}

/// Returns dimensions whose resolution need to be fetched in order to check entity vector length
/// for all arguments.
fn get_all_required_dims<'a>(
    job: &'a JobConfig,
    dimensions: &'a DimensionConfigMap,
) -> IndexSet<RequiredDim<'a>> {
    job.from
        .iter()
        .flat_map(|arg| get_required_dims(arg, dimensions))
        .collect()
}

/// Generates the code that fetches and inserts the resolution entries into the resolution map.
fn resolution_inserts(
    required_dims: &IndexSet<RequiredDim>,
    dimensions: &DimensionConfigMap,
) -> impl Iterator<Item = proc_macro2::TokenStream> {
    required_dims.iter().map(|dim| {
        let operon = operon_ident();
        let dim_meta = dimension_metadata_ident(&dim.config.id);
        let res_map = resolution_map_ident(dim);

        let get_args = dim
            .config
            .depends_on
            .iter()
            .map(clear_span);
        let dep_vars = dim.fetched_over.iter().cloned().map(clear_span);

        let dim_id = dim.config.id.to_string();
        let dep_names = dim.config.depends_on.iter().map(|d| d.to_string());
        let dep_values = dim.config.depends_on.iter().map(clear_span);

        dim.fetched_over.iter().rfold(
            quote! {
                let Some(resolution) = client.resolution(metadata::#dim_meta()).get([#(#get_args),*]).await? else {
                    return Err(#operon::error::MetaStorageError::MissingResolution {
                        dim: #dim_id,
                        deps: vec![#( (#dep_names, #dep_values) ),*],
                    }.into());
                };
                #res_map.insert([#(#dep_vars),*], resolution.ub);
            },
            |acc, dep| {
                let Some(dep_config) = dimensions.get(*dep) else {
                    panic!("Dimension `{dep}` not found in config");
                };
                let dep = clear_span(dep);
                let dep_required = dim.parent(dep_config);
                let dep_res_map = resolution_map_ident(&dep_required);
                let dep_ub_key = dep_required.fetched_over.iter().cloned().map(clear_span);

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

    let entity_name = arg_entity.id.to_string();
    let dim_names = arg_entity.dims.iter().map(|d| d.to_string());
    let dim_values = arg_entity.dims.iter().map(clear_span);

    parse_quote! {
        let Some(#arg_ident) = storage
            .#get_ident([#(#get_args),*])
            .await?
        else {
            return Err(#operon::error::StorageError::EntityNotFound {
                entity: #entity_name,
                dims: vec![#( (#dim_names, #dim_values) ),*],
            }.into());
        };
    }
}

/// Generates the code for defining a collected argument entity in the `run_job` function.
fn arg_def_collected(
    job: &JobConfig,
    entity: &EntityConfig,
    over: &[syn::Ident],
    dimensions: &DimensionConfigMap,
) -> syn::Stmt {
    let operon = operon_ident();
    let arg_ident = entity_over_dim_ident(&entity.id, over);
    let get_ident = batch_get_entity_ident(&entity.id, over);

    // Only arg dimensions that are not part of the `over` dimensions are passed to the get function
    // These dimensions are expected to be present in the job struct
    let get_args = entity
        .dims
        .iter()
        .filter(|d| !over.contains(d))
        .map(clear_span)
        .collect::<Vec<_>>();

    let check_ub = over.iter().rfold(quote! { Ok(elem) }, |acc, dim| {
        let Some(dim_config) = dimensions.get(dim) else {
            panic!("Dimension `{dim}` not found in config");
        };
        let dim = clear_span(dim);
        let dim_required = RequiredDim::new(dim_config, over);

        let res_map_ident = resolution_map_ident(&dim_required);
        let res_map_key = dim_required.fetched_over.iter().cloned().map(clear_span);

        let entity_name = entity.id.to_string();
        let dim_entries: Vec<proc_macro2::TokenStream> = entity
            .dims
            .iter()
            .map(|arg_dim| {
                let name = arg_dim.to_string();
                if *arg_dim == dim {
                    quote! { (#name, #operon::error::DimState::Aggregated) }
                } else if dim_config.depends_on.contains(arg_dim) || job.dims.contains(arg_dim) {
                    let arg_dim = clear_span(arg_dim);
                    quote! { (#name, #operon::error::DimState::Value(#arg_dim)) }
                } else {
                    quote! { (#name, #operon::error::DimState::Unresolved) }
                }
            })
            .collect();

        quote! {
            let len = elem.len();
            let ub = #res_map_ident.get(&[#(#res_map_key,)*]).unwrap_or(&0); // TODO: Handle this better
            if len < *ub {
                return Err(#operon::error::StorageError::EntityLengthMismatch {
                    entity: #entity_name,
                    dims: vec![#( #dim_entries ),*],
                    expected: *ub,
                    actual: len,
                }.into());
            }
            elem.into_iter()
                .take(*ub)
                .enumerate()
                .map(|(#dim, elem)| { #acc })
                .collect::<Result<Vec<_>, #operon::error::SchedulerError<MSto::Error>>>()
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
///     meta_storage: MSto,
///     job: Self::Job,
/// ) -> Result<Self::Resolution, operon::error::SchedulerError<MSto::Error>> {
///     let [i] = job.coordinate;
///
///     let Some(a) = storage.get_a([i]).await? else {
///         return Err(
///             operon::error::StorageError::EntityNotFound {
///                 entity: "A",
///                 dims: vec![("i", i)],
///             }
///             .into(),
///         );
///     };
///
///     let b_j = service
///         .beta(a)
///         .await
///         .map_err(operon::error::SchedulerError::UserError)?;
///     let entity = operon::Entity {
///         coordinate: job.coordinate,
///         value: b_j,
///     };
///     let resolution = operon::__private::Resolution::new(entity.value.len(), job.coordinate);
///
///     storage.put_all_b(entity).await?;
///
///     let mut conn = meta_storage.worker_conn().await?;
///     let tx = conn.transaction().await?;
///
///     tx.as_client()
///         .resolution(self.spawn_dim_meta())
///         .put(resolution)
///         .await?;
///     tx.as_client()
///         .ticket(self.job_meta())
///         .mark_done(job)
///         .await?;
///     tx.commit().await?;
///
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
    let required_dims = get_all_required_dims(job, dimensions);

    let job_coord_vars = job_dim_set.iter().cloned().map(clear_span);
    let resolution_defs = required_dims.iter().map(|dim| -> syn::Stmt {
        let res_map_var = resolution_map_ident(dim);
        let n = dim.fetched_over.len();
        let ty: syn::Type = parse_quote! {
            std::collections::HashMap<[usize; #n], usize>
        };
        parse_quote! { let mut #res_map_var: #ty = Default::default(); }
    });
    let resolution_inserts = resolution_inserts(&required_dims, dimensions);

    let maybe_define_resolutions: Option<syn::Stmt> = (!required_dims.is_empty()).then(|| {
        let res_map_vars = required_dims.iter().map(resolution_map_ident);
        let res_map_vars_clone = res_map_vars.clone();
        parse_quote! {
            let (#(#res_map_vars),*) = {
                #(#resolution_defs)*

                let conn = meta_storage.worker_conn().await?;
                let client = conn.as_client();

                #(#resolution_inserts)*

                (#(#res_map_vars_clone),*)
            };
        }
    });

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
            arg_def_collected(job, entity, &arg.over, dimensions)
        }
    });

    let result_ident = entity_over_dim_ident(&job.to, job.spawn_dim.as_slice());
    let put_fn_name = match &job.spawn_dim {
        Some(_) => batch_put_entity_ident(&job.to),
        None => put_entity_ident(&job.to),
    };

    let entity: syn::Expr = parse_quote! {
        #operon::Entity {
            coordinate: job.coordinate,
            value: #result_ident,
        }
    };
    let resolution: syn::Expr = if job.spawn_dim.is_some() {
        parse_quote! { #operon::__private::Resolution::new(entity.value.len(), job.coordinate)  }
    } else {
        parse_quote! { () }
    };

    let maybe_put_resolution: Option<syn::Stmt> = job.spawn_dim.is_some().then(|| {
        parse_quote! {
            tx.as_client().resolution(self.spawn_dim_meta()).put(resolution).await?;
        }
    });

    parse_quote! {
        #[allow(unused_variables)]
        async fn run_job(
            &self,
            service: &Svc,
            storage: &Sto,
            meta_storage: MSto,
            job: Self::Job,
        ) -> Result<Self::Resolution, #operon::error::SchedulerError<MSto::Error>> {
            let [#(#job_coord_vars),*] = job.coordinate;

            #maybe_define_resolutions;

            #(#arg_defs)*

            let #result_ident = service
                .#job_fn_name(#(#args),*)
                .await
                .map_err(operon::error::SchedulerError::UserError)?;
            let entity = #entity;
            let resolution = #resolution;

            storage.#put_fn_name(entity).await?;
            let mut conn = meta_storage.worker_conn().await?;
            let tx = conn.transaction().await?;
            #maybe_put_resolution;
            tx.as_client()
                .ticket(self.job_meta())
                .mark_done(job)
                .await?;
            tx.commit().await?;
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
