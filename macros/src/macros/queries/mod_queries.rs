use syn::parse_quote;

use crate::configs::AllConfig;
use crate::dependency_analysis::get_quota_required_dims;
use crate::macros::queries::resolution::resolution_queries;
use crate::macros::queries::ticket::ticket_queries;

/// Generates the `mod queries` module with all query-related items.
pub fn mod_queries(all_configs: &AllConfig) -> syn::ItemMod {
    let resolution_queries = all_configs.dimensions.values().map(resolution_queries);
    let ticket_queries = all_configs.jobs.values().map(|job| {
        let job_dims = job
            .dims
            .iter()
            .filter_map(|dim_id| all_configs.dimensions.get(dim_id))
            .collect::<Vec<_>>();
        let quota_required_dims =
            get_quota_required_dims(job, &all_configs.jobs, &all_configs.dimensions);
        ticket_queries(job, &job_dims, &quota_required_dims)
    });

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
