use syn::parse_quote;

use crate::{
    AllConfig,
    macros::{
        core::mod_core, prelude::prelude, queries::mod_queries, schema::mod_schema, spec::mod_spec,
        storage::mod_storage,
    },
};

pub fn operon(all_configs: &AllConfig) -> syn::File {
    let mod_core = mod_core(all_configs);
    let mod_queries = mod_queries(all_configs);
    let mod_schema = mod_schema(all_configs);
    let mod_spec = mod_spec(all_configs);
    let mod_storage = mod_storage(all_configs);

    let prelude = prelude(all_configs);

    parse_quote! {
        #mod_core
        #mod_queries
        #mod_schema
        #mod_spec
        #mod_storage

        #prelude
        pub use prelude::*;
    }
}
