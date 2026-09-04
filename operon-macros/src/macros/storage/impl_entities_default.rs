use syn::parse_quote;

use crate::configs::EntityConfigMap;
use crate::utils::{entities_ident, entity_metadata_ident, to_snake_case};

/// Generates an implementation of `Default` trait for the entities struct.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/storage/impl_entities_default.rs"))]
/// ```
pub fn impl_entities_default(service_id: &syn::Ident, entities: &EntityConfigMap) -> syn::ItemImpl {
    let entities_ident = entities_ident(service_id);
    let fields = entities.values().map(|entity| -> syn::FieldValue {
        let field_ident = to_snake_case(&entity.id);
        let entity_meta = entity_metadata_ident(&entity.id);

        parse_quote! {
            #field_ident: metadata::#entity_meta()
        }
    });

    parse_quote! {
        impl Default for #entities_ident {
            fn default() -> Self {
                #entities_ident {
                    #(#fields,)*
                }
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
        let item = impl_entities_default(&service_id, &all_entities);
        assert_item_eq(&item, "storage/impl_entities_default.rs");
    }
}
