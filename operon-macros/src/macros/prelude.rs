use syn::parse_quote;

use crate::configs::AllConfig;
use crate::utils::{service_trait_ident, sql_storage_ident, storage_trait_ident};

pub fn prelude(all_configs: &AllConfig) -> syn::ItemMod {
    let service_trait = service_trait_ident(&all_configs.service_id);
    let storage_trait = storage_trait_ident(&all_configs.service_id);
    let sql_storage = sql_storage_ident(&all_configs.service_id);

    parse_quote! {
        mod prelude {
            pub use super::traits::{#service_trait, #storage_trait};
            pub use super::storage::psql::{#sql_storage};
        }
    }
}
