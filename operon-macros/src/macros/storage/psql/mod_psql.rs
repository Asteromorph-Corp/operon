use syn::parse_quote;

use crate::configs::AllConfig;
use crate::macros::storage::psql::impl_entities_queries::impl_entities_queries;
use crate::macros::storage::psql::impl_service_storage::impl_service_storage;
use crate::macros::storage::psql::storage_definition::storage_definition;

/// Generates the `psql` module for psql storage struct.
pub fn mod_psql(all_configs: &AllConfig) -> syn::ItemMod {
    let data_storage_definition = storage_definition(&all_configs.service_id);

    let impl_entities_queries =
        impl_entities_queries(&all_configs.service_id, &all_configs.entities);
    let impl_service_storage = impl_service_storage(
        &all_configs.service_id,
        &all_configs.tasks,
        &all_configs.entities,
    );

    parse_quote! {
        pub mod psql {
            use super::*;

            #data_storage_definition

            #impl_entities_queries
            #impl_service_storage
        }
    }
}
