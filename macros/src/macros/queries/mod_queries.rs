use syn::parse_quote;

use crate::{
    AllConfig,
    macros::queries::{resolution::resolution_queries, ticket::ticket_queries},
};

/// Generates the `mod queries` module with all query-related items.
pub fn mod_queries(all_configs: &AllConfig) -> syn::ItemMod {
    let resolution_queries = all_configs.dimensions.values().map(resolution_queries);
    let ticket_queries = all_configs.jobs.values().map(ticket_queries);

    parse_quote! {
        mod queries {
            use super::*;

            mod resolution {
                use super::*;
                #(#resolution_queries)*
            }

            mod ticket {
                use super::*;
                #(#ticket_queries)*
            }

            pub use resolution::*;
            pub use ticket::*;
        }
    }
}
