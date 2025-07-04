use syn::parse_quote;

use crate::{
    DimensionConfig, JobConfig,
    macros::queries::{resolution::resolution_queries, ticket::ticket_queries},
};

pub fn mod_queries(dimensions: Vec<DimensionConfig>, jobs: Vec<JobConfig>) -> syn::ItemMod {
    let resolution_queries = dimensions.iter().map(resolution_queries);
    let ticket_queries = jobs.iter().map(ticket_queries);

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
        }
    }
}
