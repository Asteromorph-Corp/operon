use quote::quote;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::macros::spec::resolution_type::resolution_type;
use crate::operon_ident;
use crate::utils::rebuilder_ident;

/// Generates a struct definition for a job rebuilder.
///
/// # Example
/// ```rust,ignore
/// #[derive(Debug)]
/// pub struct BetaRebuilder {
///     job_meta: operon::__private::JobMetadata<1usize>,
///     spawn_dim_meta: operon::__private::DimensionMetadata<1usize>,
///     data: Vec<(
///         operon::__private::Job<1usize>,
///         operon::__private::Resolution<1usize>,
///     )>,
///     progress: operon::__private::SharedProgress,
/// }
/// ```
pub fn task_rebuilder_definition(job: &JobConfig) -> syn::ItemStruct {
    let operon = operon_ident();
    let rebuilder_ident = rebuilder_ident(&job.id);
    let resolution_type = resolution_type(job);
    let n = job.dims.len();

    let maybe_spawn_dim_meta = job.spawn_dim.is_some().then(|| {
        quote! { spawn_dim_meta: #operon::__private::DimensionMetadata<#n>, }
    });

    parse_quote! {
        #[derive(Debug)]
        pub struct #rebuilder_ident {
            job_meta: #operon::__private::JobMetadata<#n>,
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
    use crate::test_utils::simple_pipeline::job_beta;

    #[rstest]
    #[case::simple(job_beta(), "spec/rebuilder/task_rebuilder_definition.rs")]
    fn test_task_rebuilder_definition(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let item = task_rebuilder_definition(&job);
        assert_item_eq(&item, fixture_path);
    }
}
