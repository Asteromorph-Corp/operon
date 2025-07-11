use syn::parse_quote;

use crate::{
    AllConfig,
    macros::schema::{
        dimension::mod_dimension, job::mod_job, resolution::mod_resolution, ticket::mod_ticket,
    },
};

/// Generates the `mod schema` module with all schema-related items.
pub fn mod_schema(all_configs: &AllConfig) -> syn::ItemMod {
    let mod_dimension = mod_dimension(&all_configs.dimensions);
    let mod_resolution = mod_resolution(&all_configs.primary_dimension, &all_configs.dimensions);
    let mod_job = mod_job(&all_configs.jobs);
    let mod_ticket = mod_ticket(
        &all_configs.jobs,
        &all_configs.primary_entity,
        &all_configs.dimensions,
    );

    parse_quote! {
        mod schema {
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
