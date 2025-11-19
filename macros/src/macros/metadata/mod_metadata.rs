use syn::parse_quote;

use crate::configs::AllConfig;
use crate::macros::metadata::dimension::dimension_metadata;
use crate::macros::metadata::job::job_metadata;

pub fn mod_metadata(all_configs: &AllConfig) -> syn::ItemMod {
    let jobs = all_configs.jobs.values().map(job_metadata);
    let dimensions = all_configs.dimensions.values().map(dimension_metadata);

    parse_quote! {
        mod metadata {
            #(#jobs)*
            #(#dimensions)*
        }
    }
}
