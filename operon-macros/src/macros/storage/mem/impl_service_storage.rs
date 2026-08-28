use syn::parse_quote;

use crate::configs::{EntityConfig, EntityConfigMap};
use crate::operon_ident;
use crate::utils::{
    get_entity_ident, mem_storage_ident, put_entity_ident, storage_trait_ident, to_snake_case,
    to_type,
};

pub(super) fn impl_service_storage(
    service_id: &syn::Ident,
    entities: &EntityConfigMap,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let mem_storage_ident = mem_storage_ident(service_id);
    let storage_ident = storage_trait_ident(service_id);

    let single_ops = single_ops(entities);

    parse_quote! {
        #[#operon::__private::async_trait::async_trait]
        impl #storage_ident for #mem_storage_ident
        {
            #(#single_ops)*
        }
    }
}

fn single_ops(entities: &EntityConfigMap) -> impl Iterator<Item = syn::ImplItemFn> {
    entities
        .values()
        .flat_map(|entity| [single_get(entity), single_put(entity)])
}

/// Generates a get function for the implementation of the storage trait.
///
/// # Example
/// ```rust,ignore
/// async fn get_b(
///     &self,
///     coordinate: [usize; 2usize],
/// ) -> operon::error::StorageResult<Option<B>, Self::Error> {
///     let b = self.b.get(&coordinate).map(|entry| entry.clone());
///     Ok(b)
/// }
/// ```
fn single_get(entity: &EntityConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let get_fn_name = get_entity_ident(&entity.id);

    let n = entity.dims.len();
    let ty = to_type(&entity.id);
    let id = to_snake_case(&entity.id);

    parse_quote! {
        async fn #get_fn_name(&self, coordinate: [usize; #n]) -> #operon::error::StorageResult<Option<#ty>, Self::Error> {
            let #id = self.#id.get(&coordinate).as_deref().cloned();
            Ok(#id)
        }
    }
}

/// Generates a put function for the implementation of the storage trait.
///
/// # Example
/// ```rust,ignore
/// async fn put_b(
///     &self,
///     entity: operon::Entity<2usize, B>,
/// ) -> operon::error::StorageResult<(), Self::Error> {
///     self.b.insert(entity.coordinate, entity.value);
///     Ok(())
/// }
/// ```
fn single_put(entity: &EntityConfig) -> syn::ImplItemFn {
    let operon = operon_ident();
    let put_fn_name = put_entity_ident(&entity.id);

    let n = entity.dims.len();
    let id = to_snake_case(&entity.id);
    let ty = to_type(&entity.id);

    parse_quote! {
        async fn #put_fn_name(&self, entity: #operon::Entity<#n, #ty>) -> #operon::error::StorageResult<(), Self::Error> {
            self.#id.insert(entity.coordinate, entity.value);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::entity_b;

    #[rstest]
    #[case::simple(entity_b(), "storage/mem/single_get.rs")]
    fn test_single_get(#[case] entity: EntityConfig, #[case] fixture_path: &str) {
        let item = single_get(&entity);
        assert_item_eq(&item, fixture_path);
    }

    #[rstest]
    #[case::simple(entity_b(), "storage/mem/single_put.rs")]
    fn test_single_put(#[case] entity: EntityConfig, #[case] fixture_path: &str) {
        let item = single_put(&entity);
        assert_item_eq(&item, fixture_path);
    }
}
