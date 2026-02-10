use syn::parse_quote;

use crate::configs::AllConfig;
use crate::macros::metadata::dimension::dimension_metadata;
use crate::macros::metadata::entity::entity_metadata;
use crate::macros::metadata::job::job_metadata;
use crate::macros::metadata::job_id::job_id;

pub fn mod_metadata(all_configs: &AllConfig) -> syn::ItemMod {
    let job_ids = all_configs.jobs.keys().map(job_id);
    let jobs = all_configs.jobs.values().map(job_metadata);
    let dimensions = all_configs.dimensions.values().map(dimension_metadata);
    let entities = all_configs.entities.values().map(entity_metadata);

    parse_quote! {
        mod metadata {
            use super::*;

            #(#job_ids)*
            #(#jobs)*
            #(#dimensions)*
            #(#entities)*
        }
    }
}
