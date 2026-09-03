use indoc::formatdoc;
use quote::{format_ident, quote};
use syn::parse_quote;

use crate::configs::{AllConfig, EntityConfigMap, TaskConfigMap};
use crate::macros::core::DocumentedFn;
use crate::utils::{
    batch_get_entity_ident, batch_put_entity_ident, clear_span, get_entity_ident, operon_ident,
    put_entity_ident, storage_trait_ident, to_type,
};

fn format_coordinate(dims: &[syn::Ident]) -> String {
    let dims = dims.iter().map(|d| d.to_string()).collect::<Vec<_>>();
    format!("[{}]", dims.join(", "))
}

fn format_dims(dims: &[syn::Ident]) -> String {
    let dims = dims.iter().map(|d| format!("`{d}`")).collect::<Vec<_>>();
    dims.join(", ")
}

/// A helper function to generate single operation functions for each entity.
///
/// # Example
/// ```rust,ignore
/// /// ```rust,ignore
/// /// async fn get_a(&self, coordinate: [usize; 1]) -> StorageResult<Option<A>, Self::Error>
/// /// ```
/// /// Reads the `A` stored at `[i]`, or `None` if that coordinate holds nothing.
/// async fn get_a(
///     &self,
///     coordinate: [usize; 1usize],
/// ) -> operon::error::StorageResult<Option<A>, Self::Error>;
///
/// /// ```rust,ignore
/// /// async fn put_a(&self, entity: Entity<1, A>) -> StorageResult<(), Self::Error>
/// /// ```
/// /// Writes the given `A` at its own coordinate `[i]`, replacing whatever is stored there.
/// async fn put_a(
///     &self,
///     entity: operon::Entity<1usize, A>,
/// ) -> operon::error::StorageResult<(), Self::Error>;
/// ```
fn single_ops(entities: &EntityConfigMap) -> impl Iterator<Item = DocumentedFn> {
    entities.values().flat_map(|entity| -> [DocumentedFn; 2] {
        let operon = operon_ident();
        let n = entity.dims.len();
        let ty = &entity.id;
        let get_fn_name = get_entity_ident(&entity.id);
        let put_fn_name = put_entity_ident(&entity.id);
        let coordinate = format_coordinate(&entity.dims);

        let get_sig = format!(
            "async fn {get_fn_name}(&self, coordinate: [usize; {n}]) -> StorageResult<Option<{ty}>, Self::Error>"
        );
        let get_doc = formatdoc! {"
            ```rust,ignore
            {get_sig}
            ```
            Reads the `{ty}` stored at `{coordinate}`, or `None` if that coordinate holds nothing.",
        };
        let get_fn = parse_quote! {
            #[doc = #get_doc]
            async fn #get_fn_name(&self, coordinate: [usize; #n]) -> #operon::error::StorageResult<Option<#ty>, Self::Error>;
        };

        let put_sig = format!(
            "async fn {put_fn_name}(&self, entity: Entity<{n}, {ty}>) -> StorageResult<(), Self::Error>"
        );
        let put_doc = formatdoc! {"
            ```rust,ignore
            {put_sig}
            ```
            Writes the given `{ty}` at its own coordinate `{coordinate}`, replacing whatever is stored there.",
        };
        let put_fn = parse_quote! {
            #[doc = #put_doc]
            async fn #put_fn_name(&self, entity: #operon::Entity<#n, #ty>) -> #operon::error::StorageResult<(), Self::Error>;
        };

        [(get_sig, get_fn), (put_sig, put_fn)]
    })
}

/// A helper function to generate batch get functions for each task.
///
/// # Example
/// ```rust,ignore
/// /// ```rust,ignore
/// /// async fn get_all_b_j(&self, coordinate: [usize; 1]) -> StorageResult<Vec<B>, Self::Error>
/// /// ```
/// /// Reads every `B` stored at `[i, j]` over `j`, counting that dimension up from `0` and
/// /// stopping at the first coordinate that holds nothing.
/// /// Defaults to walking `get_b` one entity at a time.
/// async fn get_all_b_j(
///     &self,
///     [i]: [usize; 1usize],
/// ) -> operon::error::StorageResult<Vec<B>, Self::Error> {
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
    tasks: &TaskConfigMap,
    entities: &EntityConfigMap,
) -> impl Iterator<Item = DocumentedFn> {
    let mut targets = tasks
        .values()
        .flat_map(|task| task.from.iter().filter(|arg| !arg.over.is_empty()))
        .collect::<Vec<_>>();

    targets.sort_by_key(|arg| (&arg.id, &arg.over));
    targets.dedup_by_key(|arg| (&arg.id, &arg.over));
    targets.into_iter().map(|arg| -> DocumentedFn {
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

        let ty = &arg.id;
        let return_ty_name = (0..arg.over.len()).fold(ty.to_string(), |acc, _| format!("Vec<{acc}>"));
        let coordinate = format_coordinate(&arg_config.dims);
        let over = format_dims(&arg.over);
        let dimensions = if arg.over.len() == 1 { "that dimension" } else { "those dimensions" };

        let sig = format!(
            "async fn {fn_name}(&self, coordinate: [usize; {n}]) -> StorageResult<{return_ty_name}, Self::Error>"
        );
        let doc = formatdoc! {"
            ```rust,ignore
            {sig}
            ```
            Reads every `{ty}` stored at `{coordinate}` over {over}, counting {dimensions} up from `0` and stopping at the first coordinate that holds nothing.
            Defaults to walking `{get_fn_name}` one entity at a time.",
        };

        let batch_get_fn = parse_quote! {
            #[doc = #doc]
            async fn #fn_name(&self, [#(#args),*]: [usize; #n]) -> #operon::error::StorageResult<#return_ty, Self::Error> {
                let final_results = #body;
                Ok(final_results.unwrap_or_default())
            }
        };

        (sig, batch_get_fn)
    })
}

/// A helper function to generate batch insert functions for each task.
///
/// # Example
/// ```rust,ignore
/// /// ```rust,ignore
/// /// async fn put_all_a(&self, entity: Entity<0, Vec<A>>) -> StorageResult<(), Self::Error>
/// /// ```
/// /// Writes a whole run of `A` at `[i]`, taking `i` from each value's position in
/// /// `entity.value`.
/// /// Defaults to walking `put_a` one entity at a time.
/// async fn put_all_a(
///     &self,
///     entity: operon::Entity<0usize, Vec<A>>,
/// ) -> operon::error::StorageResult<(), Self::Error> {
///     let [] = entity.coordinate;
///     for (i, value) in entity.value.into_iter().enumerate() {
///         let entity_single = operon::Entity {
///             coordinate: [i],
///             value,
///         };
///         self.put_a(entity_single).await?;
///     }
///     Ok(())
/// }
/// ```
fn batch_inserts(tasks: &TaskConfigMap) -> impl Iterator<Item = DocumentedFn> {
    tasks.values().filter_map(|task| -> Option<DocumentedFn> {
        let operon = operon_ident();
        let fn_name = batch_put_entity_ident(&task.to);
        let n = task.dims.len();
        let ty = to_type(&task.to);

        let put_fn_name = put_entity_ident(&task.to);
        let coord_vars = task.dims.iter().map(clear_span).collect::<Vec<_>>();
        let spawn_dim = clear_span(task.spawn_dim.as_ref()?);

        let entity_id = &task.to;
        let coordinate =
            format_coordinate(&[coord_vars.as_slice(), std::slice::from_ref(&spawn_dim)].concat());
        let sig = format!(
            "async fn {fn_name}(&self, entity: Entity<{n}, Vec<{entity_id}>>) -> StorageResult<(), Self::Error>"
        );
        let doc = formatdoc! {"
            ```rust,ignore
            {sig}
            ```
            Writes a whole run of `{entity_id}` at `{coordinate}`, taking `{spawn_dim}` from each value's position in `entity.value`.
            Defaults to walking `{put_fn_name}` one entity at a time.",
        };

        let batch_insert_fn = parse_quote! {
            #[doc = #doc]
            async fn #fn_name(&self, entity: #operon::Entity<#n, Vec<#ty>>) -> #operon::error::StorageResult<(), Self::Error> {
                let [#(#coord_vars),*] = entity.coordinate;

                for (#spawn_dim, value) in entity.value.into_iter().enumerate() {
                    let entity_single = #operon::Entity {
                        coordinate: [#(#coord_vars,)* #spawn_dim],
                        value,
                    };
                    self.#put_fn_name(entity_single).await?;
                }
                Ok(())
            }
        };

        Some((sig, batch_insert_fn))
    })
}

/// Generates the storage trait for the pipeline.
pub fn trait_storage(all_configs: &AllConfig) -> syn::ItemTrait {
    let operon = operon_ident();
    let storage_ident = storage_trait_ident(&all_configs.service_id);

    let (required_sigs, single_ops) =
        single_ops(&all_configs.entities).unzip::<_, _, Vec<_>, Vec<_>>();
    let (batch_get_sigs, batch_gets) =
        batch_gets(&all_configs.tasks, &all_configs.entities).unzip::<_, _, Vec<_>, Vec<_>>();
    let (batch_insert_sigs, batch_inserts) =
        batch_inserts(&all_configs.tasks).unzip::<_, _, Vec<_>, Vec<_>>();

    let mut sections = vec![formatdoc! {"
        Generated trait containing the entity accessors that should be implemented for use with Operon.

        Implement it alongside `{operon}::OperonStorage`, which covers the backend's lifecycle and its run footprint.
        That implementation declares the `Self::Error` these methods report failure as.
        A coordinate addresses one entity across the pipeline's dimensions, and `{operon}::Entity` pairs a coordinate with the value stored there.

        # Required methods
        ```rust,ignore
        {}
        ```",
        required_sigs.join("\n"),
    }];

    let provided_sigs = [batch_get_sigs, batch_insert_sigs].concat();
    if !provided_sigs.is_empty() {
        sections.push(formatdoc! {"
            # Provided methods
            Each of these covers a whole range of one entity in a single call.
            They default to walking the accessors above one entity at a time; override them wherever the backend can serve the range in one query.
            The return value of `get_all_*` should be ordered by the dimensions they iterate over.
            ```rust,ignore
            {}
            ```",
            provided_sigs.join("\n"),
        });
    }

    let doc_comment = sections.join("\n\n");

    parse_quote! {
        #[doc = #doc_comment]
        #[#operon::__private::async_trait::async_trait]
        pub trait #storage_ident: #operon::OperonStorage {
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
    use crate::configs::{EntityConfigMap, TaskConfigMap};
    use crate::test_utils::complicated_pipeline::{
        all_entities as all_entities_complicated, all_tasks as all_tasks_complicated,
    };
    use crate::test_utils::simple_pipeline::{all_entities, all_tasks, simple_pipeline};
    use crate::test_utils::{assert_item_eq, assert_items_eq_in_trait};

    #[rstest]
    fn test_single_ops(all_entities: EntityConfigMap) {
        let items = single_ops(&all_entities)
            .map(|(_, item)| item)
            .collect::<Vec<_>>();
        assert_items_eq_in_trait(&items, "core/storage_single_ops.rs");
    }

    #[rstest]
    #[case::simple(all_tasks(), all_entities(), "core/storage_batch_gets.simple.rs")]
    #[case::multiple_over(
        all_tasks_complicated(),
        all_entities_complicated(),
        "core/storage_batch_gets.multiple_over.rs"
    )]
    fn test_batch_gets(
        #[case] all_tasks: TaskConfigMap,
        #[case] all_entities: EntityConfigMap,
        #[case] fixture_path: &str,
    ) {
        let item = batch_gets(&all_tasks, &all_entities)
            .map(|(_, item)| item)
            .collect::<Vec<_>>();
        assert_items_eq_in_trait(&item, fixture_path);
    }

    #[rstest]
    fn test_batch_inserts(all_tasks: TaskConfigMap) {
        let items = batch_inserts(&all_tasks)
            .map(|(_, item)| item)
            .collect::<Vec<_>>();
        assert_items_eq_in_trait(&items, "core/storage_batch_inserts.rs");
    }

    #[rstest]
    fn test_trait_storage(simple_pipeline: AllConfig) {
        let result = trait_storage(&simple_pipeline);
        assert_item_eq(&result, "core/storage.rs");
    }
}
