use syn::parse_quote;

use crate::configs::EntityConfigMap;
use crate::operon_ident;
use crate::utils::{entities_ident, to_snake_case};

/// Generates the implementation of `EntityQueries` trait for the entity struct.
///
/// # Example
/// ```rust,ignore
/// #[operon::__private::async_trait::async_trait]
/// impl operon::__private::EntityQueries for CookingEntities {
///     async fn init(
///         &self,
///         client: &operon::__private::StorageClient<'_>,
///     ) -> operon::error::StorageResult<(), operon::error::PsqlStorageError> {
///         self.a.init(client).await?;
///         self.b.init(client).await?;
///         self.c.init(client).await?;
///         self.d.init(client).await?;
///         self.e.init(client).await?;
///         self.f.init(client).await?;
///         Ok(())
///     }
/// }
/// ```
pub fn impl_entities_queries(service_id: &syn::Ident, entities: &EntityConfigMap) -> syn::ItemImpl {
    let operon = operon_ident();
    let entities_ident = entities_ident(service_id);
    let fields = entities.keys().map(to_snake_case).collect::<Vec<_>>();

    parse_quote! {
        #[#operon::__private::async_trait::async_trait]
        impl #operon::__private::EntityQueries for #entities_ident {
            async fn init(
                &self,
                client: &#operon::__private::StorageClient<'_>,
            ) -> #operon::error::StorageResult<(), #operon::error::PsqlStorageError> {
                #(self.#fields.init(client).await?;)*
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_entities, service_id};

    #[rstest]
    fn test_impl_entities_default(service_id: syn::Ident, all_entities: EntityConfigMap) {
        let item = impl_entities_queries(&service_id, &all_entities);
        assert_item_eq(&item, "storage/impl_entities_queries.rs");
    }
}
