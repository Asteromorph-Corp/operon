use syn::parse_quote;

use crate::{
    AllConfig,
    utils::{job_enum_ident, operon_ident, resolution_enum_ident, service_trait_ident},
};

/// Generates a trait for the service based on the provided AllConfig.
// TODO: Add methods to the service trait based on the AllConfig.
pub fn trait_service(all_configs: &AllConfig) -> syn::ItemTrait {
    let operon = operon_ident();
    let job_enum_ident = job_enum_ident();
    let res_enum_ident = resolution_enum_ident();
    let svc_ident = service_trait_ident(&all_configs.service_id);

    parse_quote! {
        #[#operon::async_trait::async_trait]
        pub trait #svc_ident: #operon::service::OperonService<JobEnum = schema::#job_enum_ident, ResolutionEnum = schema::#res_enum_ident> {
        }
    }
}
