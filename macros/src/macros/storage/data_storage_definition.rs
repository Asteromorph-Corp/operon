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
