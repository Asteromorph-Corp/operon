use syn::parse_quote;

use crate::JobConfig;
use crate::utils::{job_enum_ident, job_ident, variant_ident};

/// Generates an implementation of `From<Job>` for the `JobEnum`.
///
/// Example:
/// ```rust,ignore
/// #[automatically_derived]
/// impl From<BetaJob> for JobEnum {
///     fn from(job: BetaJob) -> Self {
///         JobEnum::Beta(job)
///     }
/// }
/// ```
pub(super) fn impl_enum_from_job(job: &JobConfig) -> syn::ItemImpl {
    let job_enum_ident = job_enum_ident();
    let job_ident = job_ident(&job.id);
    let variant_ident = variant_ident(&job.id);

    parse_quote! {
        #[automatically_derived]
        impl From<#job_ident> for #job_enum_ident {
            fn from(job: #job_ident) -> Self {
                #job_enum_ident::#variant_ident(job)
            }
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
    #[case::simple(job_beta(), "schema/job/impl_enum_from_job.rs")]
    fn test_impl_enum_from_job(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let result = impl_enum_from_job(&job);
        assert_item_eq(&result, fixture_path);
    }
}
