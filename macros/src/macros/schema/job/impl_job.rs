use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::job_ident;

/// Generates an implementation of the `Job` trait for the given job configuration.
///
/// Example:
/// ```rust,ignore
/// #[automatically_derived]
/// impl operon::schema_base::Job for BetaJob {
/// }
/// ```
pub(super) fn impl_job(job: &JobConfig) -> syn::ItemImpl {
    let operon = crate::utils::operon_ident();
    let job_ident = job_ident(&job.id);

    parse_quote! {
        #[automatically_derived]
        impl #operon::schema_base::Job for #job_ident {}
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::configs::JobConfig;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::job_beta;

    #[rstest]
    #[case::simple(job_beta(), "schema/job/impl_job.rs")]
    fn test_impl_job(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let result = impl_job(&job);
        assert_item_eq(&result, fixture_path);
    }
}
