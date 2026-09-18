use syn::parse_quote;

use crate::configs::AllConfig;
use crate::macros::storage::entities_definition::entities_definition;
use crate::macros::storage::impl_entities_default::impl_entities_default;
use crate::macros::storage::mem::mod_mem;
use crate::macros::storage::psql::mod_psql;

/// Generates the `storage` module with storage implementations.
pub(crate) fn mod_storage(all_configs: &AllConfig) -> syn::ItemMod {
    let entities_definition = entities_definition(&all_configs.service_id, &all_configs.entities);
    let impl_entities_default =
        impl_entities_default(&all_configs.service_id, &all_configs.entities);

    let mod_mem = mod_mem(all_configs);
    let mod_psql = mod_psql(all_configs);

    parse_quote! {
        mod storage {
            use super::*;

            #mod_mem
            #mod_psql
            // TODO: re-export for backwards compatibility, remove in future breaking version bump
            pub use psql::*;

            #entities_definition
            #impl_entities_default
        }
    }
}
