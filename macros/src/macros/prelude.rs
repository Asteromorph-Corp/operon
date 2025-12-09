use syn::parse_quote;

use crate::configs::AllConfig;
use crate::utils::{
    get_handler_ident, operon_ident, service_trait_ident, spec_ident, sql_storage_ident,
    storage_trait_ident,
};

pub fn prelude(all_configs: &AllConfig) -> syn::ItemMod {
    let operon = operon_ident();
    let handler_ident = get_handler_ident(&all_configs.service_id);
    let service_trait = service_trait_ident(&all_configs.service_id);
    let storage_trait = storage_trait_ident(&all_configs.service_id);
    let sql_storage = sql_storage_ident(&all_configs.service_id);

    let specs = all_configs.jobs.keys().map(spec_ident);

    parse_quote! {
        mod prelude {
            use super::*;

            pub use traits::{#service_trait, #storage_trait};
            pub use storage::{#sql_storage};

            pub fn #handler_ident<Svc: #service_trait, Sto: #storage_trait>() -> #operon::scheduler::SchedulerHandler<Svc, Sto> {
                #operon::scheduler::SchedulerHandler::new(vec![
                    #(spec::#specs.into_handler(),)*
                ])
            }
        }
    }
}
