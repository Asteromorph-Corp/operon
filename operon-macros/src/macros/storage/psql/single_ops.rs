use syn::parse_quote;

use crate::configs::{EntityConfig, EntityConfigMap};
use crate::operon_ident;
use crate::utils::{get_entity_ident, put_entity_ident, to_snake_case, to_type};

/// Generates a psql get function for an entity.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/storage/psql/single_get.rs") )]
/// ```
fn single_get(entity: &EntityConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let get_fn_name = get_entity_ident(&entity.id);

    let n = entity.dims.len();
    let ty = to_type(&entity.id);
    let id = to_snake_case(&entity.id);

    parse_quote! {
        async fn #get_fn_name(&self, coordinate: [usize; #n]) -> #operon::error::StorageResult<Option<#ty>, Self::Error> {
            self
                .conn()
                .await?
                .entity(self.entities_meta.#id)
                .get(coordinate)
                .await
        }
    }
}

/// Generates a psql put function for an entity.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/storage/psql/single_put.rs") )]
/// ```
fn single_put(entity: &EntityConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let put_fn_name = put_entity_ident(&entity.id);

    let n = entity.dims.len();
    let id = to_snake_case(&entity.id);
    let ty = to_type(&entity.id);

    parse_quote! {
        async fn #put_fn_name(&self, entity: #operon::Entity<#n, #ty>) -> #operon::error::StorageResult<(), Self::Error> {
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
    #[case::simple(entity_b(), "storage/psql/single_get.rs")]
    fn test_single_get(#[case] entity: EntityConfig, #[case] fixture_path: &str) {
        let item = single_get(&entity);
        assert_item_eq(&item, fixture_path);
    }

    #[rstest]
    #[case::simple(entity_b(), "storage/psql/single_put.rs")]
    fn test_single_put(#[case] entity: EntityConfig, #[case] fixture_path: &str) {
        let item = single_put(&entity);
        assert_item_eq(&item, fixture_path);
    }
}
