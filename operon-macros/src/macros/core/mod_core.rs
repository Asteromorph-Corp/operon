use syn::parse_quote;

use crate::configs::AllConfig;
use crate::macros::core::trait_service::trait_service;
use crate::macros::core::trait_storage::trait_storage;

pub fn mod_core(all_configs: &AllConfig) -> syn::ItemMod {
    let svc_trait = trait_service(all_configs);
    let sto_trait = trait_storage(all_configs);

    parse_quote! {
        mod traits {
            use super::*;

            #svc_trait
            #sto_trait
        }
    }
}
