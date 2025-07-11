use syn::parse_quote;

use crate::{
    AllConfig,
    macros::{
        core::{trait_service, trait_storage},
        queries::mod_queries,
        schema::mod_schema,
        spec::mod_spec,
    },
};

pub fn operon(all_configs: &AllConfig) -> syn::File {
    let mod_queries = mod_queries(all_configs);
    let mod_schema = mod_schema(all_configs);
    let mod_spec = mod_spec(all_configs);

    let svc_trait = trait_service(all_configs);
    let sto_trait = trait_storage(all_configs);

    parse_quote! {
        #mod_queries
        #mod_schema
        #mod_spec

        #svc_trait
        #sto_trait
    }
}
