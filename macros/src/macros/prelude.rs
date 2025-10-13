use syn::parse_quote;

use crate::configs::AllConfig;
use crate::utils::{
    entity_ident, get_handler_ident, operon_ident, service_trait_ident, spec_ident,
    sql_storage_ident, storage_trait_ident,
};

pub fn prelude(all_configs: &AllConfig) -> syn::ItemMod {
    let operon = operon_ident();
    let handler_ident = get_handler_ident(&all_configs.service_id);
    let service_trait = service_trait_ident(&all_configs.service_id);
    let storage_trait = storage_trait_ident(&all_configs.service_id);
    let sql_storage = sql_storage_ident(&all_configs.service_id);

    let specs = all_configs.jobs.keys().map(spec_ident);
    let generics = all_configs.entities.keys().map(entity_ident);

    parse_quote! {
        mod prelude {
            use super::*;

            pub use traits::{#service_trait, #storage_trait};

            /// The SQL storage that can be used with the service.
            ///
            /// This can only be used when all entities implement `Serialize` and `DeserializeOwned`.
            pub type #sql_storage = storage::#sql_storage<#(#generics),*>;

            pub fn #handler_ident<Svc: #service_trait, Sto: #storage_trait>() -> #operon::scheduler::SchedulerHandler<Svc, Sto> {
                #operon::scheduler::SchedulerHandler {
                    primary_handler: Box::new(spec::PrimarySpec),
                    job_handlers: vec![
                        #(Box::new(spec::#specs),)*
                    ]
                }
            }
        }
    }
}
