use syn::parse_quote;

use crate::configs::{EntityConfig, EntityConfigMap};
use crate::operon_ident;
use crate::utils::{entity_ident, get_entity_ident, put_entity_ident, variable_ident};

fn single_get(entity: &EntityConfig) -> syn::ImplItemFn {
    let entity_ident = entity_ident(&entity.id);
    let id = variable_ident(&entity.id);
    let get_fn_name = get_entity_ident(&entity.id);

    let n = entity.dims.len();

    parse_quote! {
        async fn #get_fn_name(&self, coordinate: [usize; #n]) -> Result<Option<#entity_ident>, operon::storage::StorageError> {
            self
                .conn()
                .await?
                .entity(self.entities_meta.#id)
                .get(coordinate)
                .await
        }
    }
}

fn single_put(entity: &EntityConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let id = variable_ident(&entity.id);
    let put_fn_name = put_entity_ident(&entity.id);

    let n = entity.dims.len();
    let t = entity_ident(&entity.id);

    parse_quote! {
        async fn #put_fn_name(&self, entity: #operon::schema::Entity<#n, #t>) -> Result<(), operon::storage::StorageError> {
            self
                .conn()
                .await?
                .entity(self.entities_meta.#id)
                .put(entity)
                .await
        }
    }
}

pub fn single_ops(entities: &EntityConfigMap) -> impl Iterator<Item = syn::ImplItemFn> {
    entities
        .values()
        .flat_map(|entity| [single_get(entity), single_put(entity)])
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::entity_b;

    #[rstest]
    #[case::simple(entity_b(), "storage/single_get.rs")]
    fn test_single_get(#[case] entity: EntityConfig, #[case] fixture_path: &str) {
        let item = single_get(&entity);
        assert_item_eq(&item, fixture_path);
    }

    #[rstest]
    #[case::simple(entity_b(), "storage/single_put.rs")]
    fn test_single_put(#[case] entity: EntityConfig, #[case] fixture_path: &str) {
        let item = single_put(&entity);
        assert_item_eq(&item, fixture_path);
    }
}
