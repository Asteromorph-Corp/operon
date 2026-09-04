use syn::parse_quote;

use crate::configs::EntityConfigMap;
use crate::operon_ident;
use crate::utils::{entities_ident, to_snake_case, to_type};

/// Generates the entities struct for the pipleine.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/storage/entities_definition.rs"))]
/// ```
pub fn entities_definition(service_id: &syn::Ident, entities: &EntityConfigMap) -> syn::ItemStruct {
    let operon = operon_ident();
    let entities_ident = entities_ident(service_id);
    let fields = entities.values().map(|entity| -> syn::Field {
        let field_ident = to_snake_case(&entity.id);
        let n = entity.dims.len();
        let ty = to_type(&entity.id);
        let meta_ty: syn::Type = parse_quote! {
            #operon::__private::EntityMetadata<#n, #ty>
        };
        parse_quote! { #field_ident: #meta_ty }
    });

    parse_quote! {
        pub struct #entities_ident {
            #(#fields,)*
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
    fn test_entities_definition(service_id: syn::Ident, all_entities: EntityConfigMap) {
        let item = entities_definition(&service_id, &all_entities);
        assert_item_eq(&item, "storage/entities_definition.rs");
    }
}
