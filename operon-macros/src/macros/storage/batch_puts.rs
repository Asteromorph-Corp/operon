use syn::parse_quote;

use crate::configs::TaskConfigMap;
use crate::utils::{batch_put_entity_ident, operon_ident, to_snake_case, to_type};

/// Generates the batch put function for the implementation of the storage trait.
///
/// # Example
/// ```rust,ignore
/// async fn put_all_a(
///     &self,
///     entity: operon::Entity<0usize, Vec<A>>,
/// ) -> operon::error::StorageResult<(), Self::Error> {
///     self.conn()
///         .await?
///         .entity(self.entities_meta.a)
///         .batch_put(entity)
///         .await
/// }
/// ```
pub fn batch_puts(tasks: &TaskConfigMap) -> impl Iterator<Item = syn::TraitItemFn> {
    tasks.values().filter_map(|task| -> Option<syn::TraitItemFn> {
        let operon = operon_ident();
        let id = to_snake_case(&task.to);
        task.spawn_dim.as_ref()?;

        let batch_put_fn_name = batch_put_entity_ident(&task.to);

        let n = task.dims.len();
        let ty = to_type(&task.to);

        Some(parse_quote! {
            async fn #batch_put_fn_name(&self, entity: #operon::Entity<#n, Vec<#ty>>) -> #operon::error::StorageResult<(), Self::Error> {
                self
                    .conn()
                    .await?
                    .entity(self.entities_meta.#id)
                    .batch_put(entity)
                    .await
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_items_eq_in_trait;
    use crate::test_utils::simple_pipeline::all_tasks;

    #[rstest]
    fn test_batch_puts(all_tasks: TaskConfigMap) {
        let items = batch_puts(&all_tasks).collect::<Vec<_>>();
        assert_items_eq_in_trait(&items, "storage/batch_puts.rs");
    }
}
