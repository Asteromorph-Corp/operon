use syn::parse_quote;

use crate::configs::AllConfig;
use crate::macros::storage::entities_definition::entities_definition;
use crate::macros::storage::impl_entities_default::impl_entities_default;
use crate::macros::storage::impl_entities_queries::impl_entities_queries;
use crate::macros::storage::impl_service_storage::impl_service_storage;
use crate::macros::storage::storage_definition::storage_definition;

pub fn mod_storage(all_configs: &AllConfig) -> syn::ItemMod {
    let entities_definition = entities_definition(&all_configs.service_id, &all_configs.entities);
    let data_storage_definition = storage_definition(&all_configs.service_id);

    let impl_entities_default =
        impl_entities_default(&all_configs.service_id, &all_configs.entities);
    let impl_entities_queries =
        impl_entities_queries(&all_configs.service_id, &all_configs.entities);
    let impl_service_storage = impl_service_storage(
        &all_configs.service_id,
        &all_configs.jobs,
        &all_configs.entities,
    );

    parse_quote! {
        mod storage {
            use super::*;

            #entities_definition
            #data_storage_definition

            #impl_entities_default
            #impl_entities_queries
            #impl_service_storage
        }
    }
}
