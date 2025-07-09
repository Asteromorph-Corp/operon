use syn::parse_quote;

use crate::{
    utils::{
        dimension_ident, entity_ident, get_entity_ident, operon_ident, put_entity_ident, storage_trait_ident, variable_ident
    }, AllConfig
};

/// Generates a trait for the storage required for the service.
///
/// Example:
/// ```rust,ignore
/// #[operon::async_trait::async_trait]
/// pub trait CookingStorage: operon::service::OperonService {
///     async fn get_a(&self, i: schema::IDim) -> Result<Option<A>, operon::storage::StorageError>;
///     async fn put_a(&self, i: schema::IDim, value: A) -> Result<(), operon::storage::StorageError>;
///
///     async fn get_b(&self, i: schema::IDim, j: schema::JDim) -> Result<Option<B>, operon::storage::StorageError>;
///     async fn put_b(&self, i: schema::IDim, j: schema::JDim, value: B) -> Result<(), operon::storage::StorageError>;
/// }
/// ```
pub fn trait_storage(all_configs: &AllConfig) -> syn::ItemTrait {
    let operon = operon_ident();
    let storage_ident = storage_trait_ident(&all_configs.service_id);

    let fns = all_configs
        .entities
        .values()
        .flat_map(|entity| -> [syn::TraitItemFn; 2] {
            let get_fn_name = get_entity_ident(&entity.id);
            let put_fn_name = put_entity_ident(&entity.id);
            let entity_ident = entity_ident(&entity.id);
            let dim_args = entity
                .dims
                .iter()
                .map(|d| -> syn::FnArg {
                    let arg_ident = variable_ident(d);
                    let arg_ty = dimension_ident(d);

                    parse_quote! { #arg_ident: schema::#arg_ty }
                })
                .collect::<Vec<_>>();
            
            let get_fn = parse_quote! {
                async fn #get_fn_name(&self, #(#dim_args),*) -> Result<Option<#entity_ident>, #operon::storage::StorageError>;
            };
            let put_fn = parse_quote! {
                async fn #put_fn_name(&self, #(#dim_args,)* value: &#entity_ident) -> Result<(), #operon::storage::StorageError>;
            };

            [get_fn, put_fn]
        });

    parse_quote! {
        #[#operon::async_trait::async_trait]
        pub trait #storage_ident: #operon::storage::OperonStorage {
            #(#fns)*
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{EntityConfig, configs::EntityConfigMap};

    use super::*;

    #[test]
    fn test_trait_storage() {
        let all_config = AllConfig {
            service_id: "Cooking".to_string(),
            primary_dimension: "i".to_string(),
            primary_entity: "a".to_string(),
            dimensions: Default::default(),
            entities: EntityConfigMap::from_iter([
                (
                    "a".to_string(),
                    EntityConfig {
                        id: "a".to_string(),
                        dims: vec!["i".to_string()],
                        body: String::new(),
                    },
                ),
                (
                    "b".to_string(),
                    EntityConfig {
                        id: "b".to_string(),
                        dims: vec!["i".to_string(), "j".to_string()],
                        body: String::new(),
                    },
                ),
            ]),
            jobs: Default::default(),
        };
        let item = trait_storage(&all_config);
        let expected: syn::ItemTrait = parse_quote! {
            #[operon::async_trait::async_trait]
            pub trait CookingStorage: operon::storage::OperonStorage {
                async fn get_a(&self, i: schema::IDim) -> Result<Option<A>, operon::storage::StorageError>;
                async fn put_a(&self, i: schema::IDim, value: &A) -> Result<(), operon::storage::StorageError>;

                async fn get_b(&self, i: schema::IDim, j: schema::JDim) -> Result<Option<B>, operon::storage::StorageError>;
                async fn put_b(&self, i: schema::IDim, j: schema::JDim, value: &B) -> Result<(), operon::storage::StorageError>;
            }
        };

        assert_eq!(item, expected);
    }
}
