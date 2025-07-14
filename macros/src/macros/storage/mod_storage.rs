use syn::parse_quote;

use crate::{
    AllConfig,
    macros::storage::{
        data_storage_definition::data_storage_definition, impl_new::impl_new,
        impl_service_storage::impl_service_storage, impl_storage::impl_storage,
    },
};

pub fn mod_storage(all_configs: &AllConfig) -> syn::ItemMod {
    let data_storage_definition =
        data_storage_definition(&all_configs.service_id, &all_configs.entities);
    let impl_new = impl_new(&all_configs.service_id, &all_configs.entities);
    let impl_storage = impl_storage(
        &all_configs.service_id,
        &all_configs.primary_entity,
        &all_configs.entities,
    );
    let impl_service_storage = impl_service_storage(
        &all_configs.service_id,
        &all_configs.jobs,
        &all_configs.entities,
    );

    parse_quote! {
        mod storage {
            use super::*;

            #data_storage_definition
            #impl_new
            #impl_storage
            #impl_service_storage
        }
    }
}
