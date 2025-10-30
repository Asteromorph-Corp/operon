use syn::parse_quote;

use crate::configs::AllConfig;
use crate::macros::schema::dimension::mod_dimension;
use crate::macros::schema::job::mod_job;
use crate::macros::schema::resolution::mod_resolution;
use crate::macros::schema::ticket::mod_ticket;

/// Generates the `mod schema` module with all schema-related items.
pub fn mod_schema(all_configs: &AllConfig) -> syn::ItemMod {
    let mod_dimension = mod_dimension(&all_configs.dimensions);
    let mod_resolution = mod_resolution(&all_configs.primary_dimension, &all_configs.dimensions);
    let mod_job = mod_job(&all_configs.jobs);
    let mod_ticket = mod_ticket(&all_configs.jobs, &all_configs.primary_entity);

    parse_quote! {
        pub mod schema {
            use super::*;

            #mod_dimension
            #mod_resolution
            #mod_job
            #mod_ticket

            pub use dimension::*;
            pub use resolution::*;
            pub use job::*;
            pub use ticket::*;
        }
    }
}
