use syn::parse_quote;

use crate::configs::AllConfig;
use crate::macros::storage::mem::impl_new::impl_new;
use crate::macros::storage::mem::impl_operon_storage::impl_operon_storage;
use crate::macros::storage::mem::impl_options_ext::impl_options_ext;
use crate::macros::storage::mem::impl_service_storage::impl_service_storage;
use crate::macros::storage::mem::storage_definition::storage_definition;
use crate::macros::storage::mem::trait_options_ext::trait_options_ext;

/// Generates the `mem` module for in-memory storage struct.
pub fn mod_mem(all_configs: &AllConfig) -> syn::ItemMod {
    let storage_definition = storage_definition(&all_configs.service_id, &all_configs.entities);

    let trait_options_ext = trait_options_ext(&all_configs.service_id);
    let impl_options_ext = impl_options_ext(&all_configs.service_id);
    let impl_new = impl_new(&all_configs.service_id);

    let impl_operon_storage = impl_operon_storage(&all_configs.service_id);
    let impl_service_storage = impl_service_storage(&all_configs.service_id, &all_configs.entities);

    parse_quote! {
        pub mod mem {
            use super::*;

            #storage_definition

            #trait_options_ext
            #impl_options_ext
            #impl_new

            #impl_operon_storage
            #impl_service_storage
        }
    }
}
