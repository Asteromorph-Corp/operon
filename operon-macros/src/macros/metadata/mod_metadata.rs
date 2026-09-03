use syn::parse_quote;

use crate::configs::AllConfig;
use crate::macros::metadata::dimension::dimension_metadata;
use crate::macros::metadata::entity::entity_metadata;
use crate::macros::metadata::task::task_metadata;
use crate::macros::metadata::task_id::task_id;

/// Generates the `metadata` module with the metadata for all items in the pipeline.
pub fn mod_metadata(all_configs: &AllConfig) -> syn::ItemMod {
    let task_ids = all_configs.tasks.keys().map(task_id);
    let tasks = all_configs.tasks.values().map(task_metadata);
    let dimensions = all_configs.dimensions.values().map(dimension_metadata);
    let entities = all_configs.entities.values().map(entity_metadata);

    parse_quote! {
        mod metadata {
            use super::*;

            #(#task_ids)*
            #(#tasks)*
            #(#dimensions)*
            #(#entities)*
        }
    }
}
