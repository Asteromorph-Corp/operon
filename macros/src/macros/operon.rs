use syn::parse_quote;

use crate::configs::AllConfig;
use crate::macros::core::mod_core;
use crate::macros::metadata::mod_metadata;
use crate::macros::prelude::prelude;
use crate::macros::queries::mod_queries;
use crate::macros::schema::mod_schema;
use crate::macros::spec::mod_spec;
use crate::macros::storage::mod_storage;

pub fn operon(all_configs: &AllConfig) -> syn::File {
    let mod_core = mod_core(all_configs);
    let mod_metadata = mod_metadata(all_configs);
    let mod_queries = mod_queries(all_configs);
    let mod_schema = mod_schema(all_configs);
    let mod_spec = mod_spec(all_configs);
    let mod_storage = mod_storage(all_configs);

    let prelude = prelude(all_configs);

    parse_quote! {
        #mod_core
        #mod_metadata
        #mod_queries
        #mod_schema
        #mod_spec
        #mod_storage

        #prelude
        pub use prelude::*;
    }
}
