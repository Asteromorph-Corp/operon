use quote::quote;
use syn::parse_quote;

use crate::configs::{EntityConfigMap, JobConfigMap};
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
/// ) -> Result<Vec<B>, operon::storage::StorageError> {
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
    jobs: &JobConfigMap,
    entities: &EntityConfigMap,
) -> impl Iterator<Item = syn::TraitItemFn> {
    let mut targets = jobs
        .values()
        .flat_map(|job| job.from.iter().filter(|arg| !arg.over.is_empty()))
        .collect::<Vec<_>>();

    targets.sort_by_key(|arg| &arg.id);
    targets.dedup_by_key(|arg| &arg.id);

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
            let i_plus_1 = i + 1;

            if i_plus_1 == arg.over.len() {
                quote! {
                    result.push(entity);
                }
            } else {
                quote! {
                    let #over = usize::try_from(row.get::<_, i64>(#i_plus_1))?; // TODO: Handle None case
                    while result.len() <= #over {
                        result.push(Default::default());
                    }
                    let mut result = &mut result[#over];
                }
            }
        });

        parse_quote! {
            async fn #batch_get_fn_name(&self, [#(#arg_dims),*]: [usize; #n]) -> Result<#return_ty, #operon::storage::StorageError> {
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
    use crate::test_utils::simple_pipeline::{all_entities, all_jobs};

    #[rstest]
    fn test_batch_gets(all_jobs: JobConfigMap, all_entities: EntityConfigMap) {
        let items = batch_gets(&all_jobs, &all_entities).collect::<Vec<_>>();
        assert_items_eq_in_trait(&items, "storage/batch_gets.rs");
    }
}
