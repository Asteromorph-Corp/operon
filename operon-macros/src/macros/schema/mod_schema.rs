use syn::parse_quote;

use crate::configs::AllConfig;
use crate::macros::schema::job::{impl_job_enum, job_enum_definition};
use crate::macros::schema::resolution::{impl_resolution_enum, resolution_enum_definition};
use crate::macros::schema::ticket::{impl_ticket_enum, ticket_enum_definition};

/// Generates the `schema` module with all schema types.
pub(crate) fn mod_schema(all_configs: &AllConfig) -> syn::ItemMod {
    let job_enum_def = job_enum_definition(&all_configs.tasks);
    let impl_job_enum = impl_job_enum();

    let resolution_enum_def = resolution_enum_definition(&all_configs.dimensions);
    let impl_resolution_enum = impl_resolution_enum();

    let ticket_enum_def = ticket_enum_definition(&all_configs.tasks);
    let impl_ticket_enum = impl_ticket_enum();

    parse_quote! {
        pub mod schema {
            use super::*;

            #job_enum_def
            #impl_job_enum

            #resolution_enum_def
            #impl_resolution_enum

            #ticket_enum_def
            #impl_ticket_enum
        }
    }
}
