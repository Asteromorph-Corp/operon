use syn::parse_quote;

use crate::utils::{operon_ident, sql_storage_ident};

pub(super) fn data_storage_definition(service_id: &str) -> syn::ItemStruct {
    let operon = operon_ident();
    let sql_storage_ident = sql_storage_ident(service_id);

    parse_quote! {
        #[derive(Debug, Clone)]
        pub struct #sql_storage_ident {
            pub pool: #operon::deadpool_postgres::Pool,
            pub schema: Option<String>,
        }
    }
}
