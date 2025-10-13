use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{job_ident, rebuilder_ident, spawn_resolution};

/// Generates a struct definition for a job rebuilder.
///
/// Example:
/// ```rust,ignore
/// #[derive(Debug)]
/// pub struct BetaRebuilder(Vec<(schema::BetaJob, schema::JResolution)>);
/// ```
pub fn job_rebuilder_definition(job: &JobConfig) -> syn::ItemStruct {
    let rebuilder_ident = rebuilder_ident(&job.id);
    let job_ident = job_ident(&job.id);
    let res_ident = spawn_resolution(job.spawn_dim.as_ref());

    parse_quote! {
        #[derive(Debug)]
        pub struct #rebuilder_ident(Vec<(schema::#job_ident, #res_ident)>);
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::job_beta;

    #[rstest]
    #[case::simple(job_beta(), "spec/job_rebuilder_definition.rs")]
    fn test_job_rebuilder_definition(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let item = job_rebuilder_definition(&job);
        assert_item_eq(&item, fixture_path);
    }
}
