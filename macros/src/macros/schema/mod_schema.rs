use syn::parse_quote;

use crate::configs::AllConfig;
use crate::macros::schema::job::mod_job;
use crate::macros::schema::resolution::mod_resolution;

/// Generates the `mod schema` module with all schema-related items.
pub fn mod_schema(all_configs: &AllConfig) -> syn::ItemMod {
    let mod_resolution = mod_resolution(&all_configs.dimensions);
    let mod_job = mod_job(&all_configs.jobs);

    parse_quote! {
        pub mod schema {
            use super::*;

            #mod_resolution
            #mod_job

            pub use resolution::*;
            pub use job::*;
        }
    }
}
