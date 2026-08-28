use syn::parse_quote;

use crate::configs::AllConfig;
use crate::utils::{
    mem_storage_ident, mem_storage_options_ext_ident, service_trait_ident, sql_storage_ident,
    storage_trait_ident,
};

pub fn prelude(all_configs: &AllConfig) -> syn::ItemMod {
    let service_trait = service_trait_ident(&all_configs.service_id);
    let storage_trait = storage_trait_ident(&all_configs.service_id);
    let mem_storage = mem_storage_ident(&all_configs.service_id);
    let mem_storage_options_ext = mem_storage_options_ext_ident(&all_configs.service_id);
    let sql_storage = sql_storage_ident(&all_configs.service_id);

    parse_quote! {
        mod prelude {
            pub use super::traits::{#service_trait, #storage_trait};
            pub use super::storage::psql::{#sql_storage};
            pub use super::storage::mem::{#mem_storage, #mem_storage_options_ext};
        }
    }
}
