use syn::parse_quote;

use crate::configs::EntityConfigMap;
use crate::operon_ident;
use crate::utils::{entities_ident, entity_ident, variable_ident};

/// Generates the entities struct to be used as a generic parameter for the `PsqlStorage`.
///
/// # Example
/// ```rust,ignore
/// pub struct CookingEntities {
///     a: operon::schema::EntityMetadata<1usize, A>,
///     b: operon::schema::EntityMetadata<2usize, B>,
///     c: operon::schema::EntityMetadata<2usize, C>,
///     d: operon::schema::EntityMetadata<3usize, D>,
///     e: operon::schema::EntityMetadata<2usize, E>,
///     f: operon::schema::EntityMetadata<1usize, F>,
/// }
/// ```
pub fn entities_definition(service_id: &str, entities: &EntityConfigMap) -> syn::ItemStruct {
    let operon = operon_ident();
    let entities_ident = entities_ident(service_id);
    let fields = entities.values().map(|entity| -> syn::Field {
        let field_ident = variable_ident(&entity.id);
        let n = entity.dims.len();
        let t = entity_ident(&entity.id);
        let field_ty: syn::Type = parse_quote! {
            #operon::schema::EntityMetadata<#n, #t>
        };

        parse_quote! {
            #field_ident: #field_ty
        }
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
    fn test_entities_definition(service_id: &str, all_entities: EntityConfigMap) {
        let item = entities_definition(service_id, &all_entities);
        assert_item_eq(&item, "storage/entities_definition.rs");
    }
}
