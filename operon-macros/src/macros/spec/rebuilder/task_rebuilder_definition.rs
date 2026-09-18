use quote::quote;
use syn::parse_quote;

use crate::configs::TaskConfig;
use crate::macros::spec::resolution_type::resolution_type;
use crate::operon_ident;
use crate::utils::rebuilder_ident;

/// Generates the task rebuilder struct for a task.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/spec/rebuilder/task_rebuilder_definition.rs"))]
/// ```
pub(crate) fn task_rebuilder_definition(task: &TaskConfig) -> syn::ItemStruct {
    let operon = operon_ident();
    let rebuilder_ident = rebuilder_ident(&task.id);
    let resolution_type = resolution_type(task);
    let n = task.dims.len();

    let maybe_spawn_dim_meta = task.spawn_dim.is_some().then(|| {
        quote! { spawn_dim_meta: #operon::__private::DimensionMetadata<#n>, }
    });

    parse_quote! {
        #[derive(Debug)]
        pub struct #rebuilder_ident {
            task_meta: #operon::__private::TaskMetadata<#n>,
            #maybe_spawn_dim_meta
            data: Vec<(#operon::__private::Job<#n>, #resolution_type)>,
            progress: #operon::__private::SharedProgress,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::task_beta;

    #[rstest]
    #[case::simple(task_beta(), "spec/rebuilder/task_rebuilder_definition.rs")]
    fn test_task_rebuilder_definition(#[case] task: TaskConfig, #[case] fixture_path: &str) {
        let item = task_rebuilder_definition(&task);
        assert_item_eq(&item, fixture_path);
    }
}
