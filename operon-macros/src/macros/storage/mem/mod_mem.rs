use syn::parse_quote;

use crate::configs::AllConfig;
use crate::macros::storage::mem::impl_operon_storage::impl_operon_storage;
use crate::macros::storage::mem::impl_service_storage::impl_service_storage;
use crate::macros::storage::mem::storage_definition::storage_definition;

pub fn mod_mem(all_configs: &AllConfig) -> syn::ItemMod {
    let storage_definition = storage_definition(&all_configs.service_id, &all_configs.entities);

    let impl_operon_storage = impl_operon_storage(&all_configs.service_id);
    let impl_service_storage = impl_service_storage(&all_configs.service_id, &all_configs.entities);

    parse_quote! {
        pub mod mem {
            use super::*;

            #storage_definition

            #impl_operon_storage
            #impl_service_storage
        }
    }
}
