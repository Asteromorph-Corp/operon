use syn::parse_quote;

use crate::{
    AllConfig,
    macros::schema::{job::mod_job, resolution::mod_resolution},
};

/// Generates the `mod schema` module with all schema-related items.
pub fn mod_schema(all_configs: &AllConfig) -> syn::ItemMod {
    let mod_resolution = mod_resolution(&all_configs.primary_dimension, &all_configs.dimensions);
    let mod_job = mod_job(&all_configs.jobs);

    parse_quote! {
        mod schema {
            use super::*;

            #mod_resolution

            #mod_job

            pub use resolution::*;
            pub use job::*;
        }
    }
}
