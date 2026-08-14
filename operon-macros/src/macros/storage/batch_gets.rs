use quote::quote;
use syn::parse_quote;

use crate::configs::{EntityConfigMap, TaskConfigMap};
use crate::utils::{
    batch_get_entity_ident, clear_span, operon_ident, to_lit_str, to_snake_case, to_type,
};

/// Generates the batch get function for the implementation of the storage trait.
///
/// # Example
/// ```rust,ignore
/// async fn get_all_b_over_j(
///     &self,
///     [i]: [usize; 1usize],
/// ) -> operon::error::StorageResult<Vec<B>, Self::Error> {
///     let entities = self
///         .conn()
///         .await?
///         .entity(self.entities_meta.b)
///         .batch_get([i], ["j"])
///         .await?;
///
///     let mut result: Vec<B> = Default::default();
///     for entity in entities {
///         result.push(entity);
///     }
///
///     Ok(result)
/// }
/// ```
pub fn batch_gets(
    tasks: &TaskConfigMap,
    entities: &EntityConfigMap,
) -> impl Iterator<Item = syn::TraitItemFn> {
    let mut targets = tasks
        .values()
        .flat_map(|task| task.from.iter().filter(|arg| !arg.over.is_empty()))
        .collect::<Vec<_>>();

    targets.sort_by_key(|arg| (&arg.id, &arg.over));
    targets.dedup_by_key(|arg| (&arg.id, &arg.over));

    targets.into_iter().map(|arg| -> syn::TraitItemFn {
        let operon = operon_ident();
        let id = to_snake_case(&arg.id);
        let batch_get_fn_name = batch_get_entity_ident(&arg.id, &arg.over);

        let arg_config = entities.get(&arg.id)
            .unwrap_or_else(|| panic!("Entity {} not found in entities", arg.id));

        let return_ty: syn::Type = arg.over.iter().fold(
            to_type(&arg.id),
            |acc, _| parse_quote! { Vec<#acc> },
        );

        let arg_dims = arg_config.dims.iter().filter(|d| !arg.over.contains(d)).map(clear_span).collect::<Vec<_>>();
        let over_dims = arg.over.iter().map(to_lit_str);

        let n = arg_dims.len();

        let insert_results = arg.over.iter().enumerate().map(|(i, over)| {
            let over = clear_span(over);

            if i + 1 == arg.over.len() {
                quote! {
                    result.push(entity.value);
                }
            } else {
                quote! {
                    let #over = entity.coordinate[#i]; // TODO: Handle None case
                    while result.len() <= #over {
                        result.push(Default::default());
                    }
                    let mut result = &mut result[#over];
                }
            }
        });

        parse_quote! {
            async fn #batch_get_fn_name(&self, [#(#arg_dims),*]: [usize; #n]) -> #operon::error::StorageResult<#return_ty, Self::Error> {
                let entities = self
                    .conn()
                    .await?
                    .entity(self.entities_meta.#id)
                    .batch_get([#(#arg_dims),*], [#(#over_dims),*])
                    .await?;

                let mut result: #return_ty = Default::default();
                for entity in entities {
                    #(#insert_results)*
                }
                Ok(result)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_items_eq_in_trait;
    use crate::test_utils::simple_pipeline::{all_entities, all_tasks};

    #[rstest]
    fn test_batch_gets(all_tasks: TaskConfigMap, all_entities: EntityConfigMap) {
        let items = batch_gets(&all_tasks, &all_entities).collect::<Vec<_>>();
        assert_items_eq_in_trait(&items, "storage/batch_gets.rs");
    }
}
