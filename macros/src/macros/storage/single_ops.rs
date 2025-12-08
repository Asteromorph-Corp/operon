use syn::parse_quote;

use crate::configs::{EntityConfig, EntityConfigMap};
use crate::operon_ident;
use crate::utils::{as_type, get_entity_ident, put_entity_ident, variable_ident};

/// Generates a get function for the implementation of the storage trait.
///
/// # Example
/// ```rust,ignore
/// async fn get_b(
///     &self,
///     coordinate: [usize; 2usize],
/// ) -> Result<Option<B>, operon::storage::StorageError> {
///     self.conn()
///         .await?
///         .entity(self.entities_meta.b)
///         .get(coordinate)
///         .await
/// }
/// ```
fn single_get(entity: &EntityConfig) -> syn::ImplItemFn {
    let get_fn_name = get_entity_ident(&entity.id);

    let n = entity.dims.len();
    let ty = as_type(&entity.id);
    let id = variable_ident(&entity.id);

    parse_quote! {
        async fn #get_fn_name(&self, coordinate: [usize; #n]) -> Result<Option<#ty>, operon::storage::StorageError> {
            self
                .conn()
                .await?
                .entity(self.entities_meta.#id)
                .get(coordinate)
                .await
        }
    }
}

/// Generates a put function for the implementation of the storage trait.
///
/// # Example
/// ```rust,ignore
/// async fn put_b(
///     &self,
///     entity: operon::schema::Entity<2usize, B>,
/// ) -> Result<(), operon::storage::StorageError> {
///     self.conn()
///         .await?
///         .entity(self.entities_meta.b)
///         .put(entity)
///         .await
/// }
/// ```
fn single_put(entity: &EntityConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let put_fn_name = put_entity_ident(&entity.id);

    let n = entity.dims.len();
    let id = variable_ident(&entity.id);
    let ty = as_type(&entity.id);

    parse_quote! {
        async fn #put_fn_name(&self, entity: #operon::schema::Entity<#n, #ty>) -> Result<(), operon::storage::StorageError> {
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
