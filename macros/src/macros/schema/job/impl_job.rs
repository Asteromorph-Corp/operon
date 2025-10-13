use syn::parse_quote;

use crate::configs::{JobConfig, JobConfigMap};
use crate::dependency_analysis::get_upstream_jobs;
use crate::utils::{job_id_ident, job_ident};

/// Generates an implementation of the `Job` trait for the given job configuration.
///
/// Example:
/// ```rust,ignore
/// #[automatically_derived]
/// impl operon::schema_base::Job for BetaJob {
///     fn id() -> &'static str {
///         BETA_ID
///     }
///
///     fn is_descendant_of(other: &str) -> bool {
///         other == BETA_ID
///     }
/// }
/// ```
pub(super) fn impl_job(job: &JobConfig, jobs: &JobConfigMap) -> syn::ItemImpl {
    let operon = crate::utils::operon_ident();
    let job_ident = job_ident(&job.id);

    let id_ident = job_id_ident(&job.id);
    let upstream_jobs = get_upstream_jobs(job, jobs)
        .into_iter()
        .map(|upstream_job| job_id_ident(&upstream_job.id));

    parse_quote! {
        #[automatically_derived]
        impl #operon::schema_base::Job for #job_ident {
            fn id() -> &'static str {
                #id_ident
            }

            fn is_descendant_of(other: &str) -> bool {
                #(
                    other == #upstream_jobs
                )||*
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::configs::{JobConfig, JobConfigMap};
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_jobs, job_beta, job_epsilon};

    #[rstest]
    #[case::simple(job_beta(), "schema/job/impl_job.simple.rs")]
    #[case::has_ancestor(job_epsilon(), "schema/job/impl_job.has_ancestor.rs")]
    fn test_impl_job(all_jobs: JobConfigMap, #[case] job: JobConfig, #[case] fixture_path: &str) {
        let result = impl_job(&job, &all_jobs);
        assert_item_eq(&result, fixture_path);
    }
}
