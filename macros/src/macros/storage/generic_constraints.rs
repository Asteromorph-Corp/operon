use syn::parse_quote;

use crate::{
    configs::EntityConfigMap,
    utils::{entity_ident, operon_ident},
};

pub(super) fn generic_constraints(
    entities: &EntityConfigMap,
) -> impl Iterator<Item = syn::WherePredicate> {
    entities
        .values()
        .map(|entity| -> syn::WherePredicate {
            let operon = operon_ident();
            let generic_param = &entity.generic;
            let entity_ident = entity_ident(&entity.id);
            parse_quote! {
                #generic_param: From<#entity_ident> + Into<#entity_ident> + #operon::serde::Serialize + #operon::serde::de::DeserializeOwned + Send + Sync + 'static
            }
        })
}
