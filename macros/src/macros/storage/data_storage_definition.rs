use syn::parse_quote;

use crate::configs::EntityConfigMap;
use crate::utils::{operon_ident, sql_storage_ident};

pub(super) fn data_storage_definition(
    service_id: &str,
    entities: &EntityConfigMap,
) -> syn::ItemStruct {
    let operon = operon_ident();
    let sql_storage_ident = sql_storage_ident(service_id);

    let generics = entities
        .values()
        .map(|entity| &entity.generic)
        .collect::<Vec<_>>();

    parse_quote! {
        #[derive(Debug, Clone)]
        pub struct #sql_storage_ident <#(#generics),*> {
            pub pool: #operon::deadpool_postgres::Pool,
            pub schema: Option<String>,
            _phantom: std::marker::PhantomData<(#(#generics),*)>,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::configs::EntityConfigMap;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_entities, service_id};

    #[rstest]
    fn test_data_storage_definition(service_id: &str, all_entities: EntityConfigMap) {
        let item = data_storage_definition(service_id, &all_entities);
        assert_item_eq(&item, "storage/data_storage_definition.rs");
    }
}
