use syn::parse_quote;

use crate::configs::JobConfigMap;
use crate::utils::{batch_put_entity_ident, entity_ident, operon_ident, variable_ident};

pub fn batch_puts(jobs: &JobConfigMap) -> impl Iterator<Item = syn::TraitItemFn> {
    jobs.values().filter_map(|job| -> Option<syn::TraitItemFn> {
        let operon = operon_ident();
        let id = variable_ident(&job.to);
        job.spawn_dim.as_ref()?;

        let batch_put_fn_name = batch_put_entity_ident(&job.to);

        let n = job.dims.len();
        let t = entity_ident(&job.to);

        Some(parse_quote! {
            async fn #batch_put_fn_name(&self, entity: #operon::schema::Entity<#n, Vec<#t>>) -> Result<(), #operon::storage::StorageError> {
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
    use crate::test_utils::simple_pipeline::all_jobs;

    #[rstest]
    fn test_batch_puts(all_jobs: JobConfigMap) {
        let items = batch_puts(&all_jobs).collect::<Vec<_>>();
        assert_items_eq_in_trait(&items, "storage/batch_puts.rs");
    }
}
