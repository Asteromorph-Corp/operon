use quote::quote;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::macros::spec::resolution_type::resolution_type;
use crate::operon_ident;
use crate::utils::rebuilder_ident;

/// Generates a struct definition for a job rebuilder.
///
/// Example:
/// ```rust,ignore
/// #[derive(Debug)]
/// pub struct BetaRebuilder(Vec<(schema::BetaJob, schema::JResolution)>);
/// ```
pub fn job_rebuilder_definition(job: &JobConfig) -> syn::ItemStruct {
    let operon = operon_ident();
    let rebuilder_ident = rebuilder_ident(&job.id);
    let resolution_type = resolution_type(job);
    let n = job.dims.len();

    let maybe_spawn_dim_meta = job.spawn_dim.is_some().then(|| {
        quote! { spawn_dim_meta: #operon::schema_base::DimensionMetadata<#n>, }
    });

    parse_quote! {
        #[derive(Debug)]
        pub struct #rebuilder_ident {
            job_meta: #operon::schema_base::JobMetadata<#n>,
            #maybe_spawn_dim_meta
            data: Vec<(#operon::schema_base::Job<#n>, #resolution_type)>,
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
    #[case::simple(job_beta(), "spec/rebuilder/job_rebuilder_definition.rs")]
    fn test_job_rebuilder_definition(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let item = job_rebuilder_definition(&job);
        assert_item_eq(&item, fixture_path);
    }
}
