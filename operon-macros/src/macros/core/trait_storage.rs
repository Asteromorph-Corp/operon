use quote::{format_ident, quote};
use syn::parse_quote;

use crate::configs::{AllConfig, EntityConfigMap, JobConfigMap};
use crate::utils::{
    batch_get_entity_ident, batch_put_entity_ident, clear_span, get_entity_ident, operon_ident,
    put_entity_ident, storage_trait_ident, to_type,
};

/// A helper function to generate single operation functions for each entity.
///
/// # Example
/// ```rust,ignore
/// async fn get_a(
///     &self,
///     coordinate: [usize; 1usize],
/// ) -> Result<Option<A>, operon::storage::StorageError>;
///
/// async fn put_a(
///     &self,
///     entity: operon::schema::Entity<1usize, A>,
/// ) -> Result<(), operon::storage::StorageError>;
/// ```
fn single_ops(entities: &EntityConfigMap) -> impl Iterator<Item = syn::TraitItemFn> {
    entities.values().flat_map(|entity| -> [syn::TraitItemFn; 2] {
        let operon = operon_ident();
        let n = entity.dims.len();
        let ty = &entity.id;
        let get_fn_name = get_entity_ident(&entity.id);
        let put_fn_name = put_entity_ident(&entity.id);

        let get_fn = parse_quote! {
            async fn #get_fn_name(&self, coordinate: [usize; #n]) -> Result<Option<#ty>, #operon::storage::StorageError>;
        };
        let put_fn = parse_quote! {
            async fn #put_fn_name(&self, entity: #operon::schema::Entity<#n, #ty>) -> Result<(), #operon::storage::StorageError>;
        };

        [get_fn, put_fn]
    })
}

/// A helper function to generate batch get functions for each job.
///
/// # Example
/// ```rust,ignore
/// async fn get_all_b_over_j(
///     &self,
///     [i]: [usize; 1usize],
/// ) -> Result<Vec<B>, operon::storage::StorageError> {
///     let final_results = {
///         let mut results_0 = Vec::new();
///         let mut j = 0usize;
///         while let Some(value) = self.get_b([i, j]).await? {
///             results_0.push(value);
///             j += 1;
///         }
///         (!results_0.is_empty()).then_some(results_0)
///     };
///     Ok(final_results.unwrap_or_default())
/// }
/// ```
fn batch_gets(
    jobs: &JobConfigMap,
    entities: &EntityConfigMap,
) -> impl Iterator<Item = syn::TraitItemFn> {
    let mut targets = jobs
        .values()
        .flat_map(|job| job.from.iter().filter(|arg| !arg.over.is_empty()))
        .collect::<Vec<_>>();

    targets.sort_by_key(|arg| (&arg.id, &arg.over));
    targets.dedup_by_key(|arg| (&arg.id, &arg.over));
    targets.into_iter().map(|arg| -> syn::TraitItemFn {
        let operon = operon_ident();

        let arg_config = entities.get(&arg.id)
            .unwrap_or_else(|| panic!("Entity {} not found in entities", arg.id));

        let fn_name = batch_get_entity_ident(&arg.id, &arg.over);
        let args = arg_config
            .dims
            .iter()
            .filter(|d| !arg.over.contains(d))
            .map(clear_span)
            .collect::<Vec<_>>();
        let return_ty: syn::Type = arg.over.iter().fold(
            to_type(&arg.id),
            |acc, _| parse_quote! { Vec<#acc> },
        );
        let n = args.len();

        let get_fn_name = get_entity_ident(&arg.id);
        let get_args = arg_config.dims.iter().map(clear_span);

        let body = arg.over.iter().enumerate().rfold(
            quote! {
                self.#get_fn_name([#(#get_args),*]).await?
            },
            |acc, (i, over)| {
                let over = clear_span(over);
                let results_ident = format_ident!("results_{i}");

                quote! {
                    {
                        let mut #results_ident = Vec::new();
                        let mut #over = 0usize;
                        while let Some(value) = #acc {
                            #results_ident.push(value);
                            #over += 1;
                        }
                        (!#results_ident.is_empty()).then_some(#results_ident)
                    }
                }
            }
        );

        parse_quote! {
            async fn #fn_name(&self, [#(#args),*]: [usize; #n]) -> Result<#return_ty, #operon::storage::StorageError> {
                let final_results = #body;
                Ok(final_results.unwrap_or_default())
            }
        }
    })
}

/// A helper function to generate batch insert functions for each job.
///
/// # Example
/// ```rust,ignore
/// async fn put_all_a(
///     &self,
///     entity: operon::schema::Entity<0usize, Vec<A>>,
/// ) -> Result<(), operon::storage::StorageError> {
///     let [] = entity.coordinate;
///     for (i, value) in entity.value.into_iter().enumerate() {
///         let entity_single = operon::schema::Entity {
///             coordinate: [i],
///             value,
///         };
///         self.put_a(entity_single).await?;
///     }
///     Ok(())
/// }
/// ```
fn batch_inserts(jobs: &JobConfigMap) -> impl Iterator<Item = syn::TraitItemFn> {
    jobs.values().filter_map(|job| -> Option<syn::TraitItemFn> {
        let operon = operon_ident();
        let fn_name = batch_put_entity_ident(&job.to);
        let n = job.dims.len();
        let ty = to_type(&job.to);

        let put_fn_name = put_entity_ident(&job.to);
        let coord_vars = job.dims.iter().map(clear_span).collect::<Vec<_>>();
        let spawn_dim = clear_span(job.spawn_dim.as_ref()?);

        Some(parse_quote! {
            async fn #fn_name(&self, entity: #operon::schema::Entity<#n, Vec<#ty>>) -> Result<(), #operon::storage::StorageError> {
                let [#(#coord_vars),*] = entity.coordinate;

                for (#spawn_dim, value) in entity.value.into_iter().enumerate() {
                    let entity_single = #operon::schema::Entity {
                        coordinate: [#(#coord_vars,)* #spawn_dim],
                        value,
                    };
                    self.#put_fn_name(entity_single).await?;
                }
                Ok(())
            }
        })
    })
}

/// Generates a trait for the storage required for the service.
pub fn trait_storage(all_configs: &AllConfig) -> syn::ItemTrait {
    let operon = operon_ident();
    let storage_ident = storage_trait_ident(&all_configs.service_id);

    let single_ops = single_ops(&all_configs.entities);
    let batch_gets = batch_gets(&all_configs.jobs, &all_configs.entities);
    let batch_inserts = batch_inserts(&all_configs.jobs);

    parse_quote! {
        #[#operon::__private::async_trait::async_trait]
        pub trait #storage_ident: #operon::storage::OperonStorage {
            #(#single_ops)*
            #(#batch_gets)*
            #(#batch_inserts)*
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::configs::{EntityConfigMap, JobConfigMap};
    use crate::test_utils::assert_items_eq_in_trait;
    use crate::test_utils::complicated_pipeline::{
        all_entities as all_entities_complicated, all_jobs as all_jobs_complicated,
    };
    use crate::test_utils::simple_pipeline::{all_entities, all_jobs};

    #[rstest]
    fn test_single_ops(all_entities: EntityConfigMap) {
        let items = single_ops(&all_entities).collect::<Vec<_>>();
        assert_items_eq_in_trait(&items, "core/storage_single_ops.rs");
    }

    #[rstest]
    #[case::simple(all_jobs(), all_entities(), "core/storage_batch_gets.simple.rs")]
    #[case::multiple_over(
        all_jobs_complicated(),
        all_entities_complicated(),
        "core/storage_batch_gets.multiple_over.rs"
    )]
    fn test_batch_gets(
        #[case] all_jobs: JobConfigMap,
        #[case] all_entities: EntityConfigMap,
        #[case] fixture_path: &str,
    ) {
        let item = batch_gets(&all_jobs, &all_entities).collect::<Vec<_>>();
        assert_items_eq_in_trait(&item, fixture_path);
    }

    #[rstest]
    fn test_batch_inserts(all_jobs: JobConfigMap) {
        let items = batch_inserts(&all_jobs).collect::<Vec<_>>();
        assert_items_eq_in_trait(&items, "core/storage_batch_inserts.rs");
    }
}
