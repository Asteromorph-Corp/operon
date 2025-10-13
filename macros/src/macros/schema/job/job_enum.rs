use syn::parse_quote;

use crate::configs::JobConfigMap;
use crate::utils::{job_enum_ident, job_ident, variant_ident};

/// Generates an enum representing all jobs in the job configuration map.
///
/// Example:
/// ```rust,ignore
/// #[doc = "An enum representing any job."]
/// #[derive(Debug, Clone)]
/// pub enum JobEnum {
///     Beta(BetaJob),
///     Gamma(GammaJob),
/// }
/// ```
pub(super) fn job_enum(jobs: &JobConfigMap) -> syn::ItemEnum {
    let job_enum_ident = job_enum_ident();
    let variants = jobs.keys().map(|job| -> syn::Variant {
        let variant_ident = variant_ident(job);
        let inner = job_ident(job);
        parse_quote! {
            #variant_ident(#inner)
        }
    });
    let doc = "An enum representing any job.";

    parse_quote! {
        #[doc = #doc]
        #[derive(Debug, Clone)]
        pub enum #job_enum_ident {
            #(#variants,)*
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::all_jobs;

    #[rstest]
    fn test_job_enum(all_jobs: JobConfigMap) {
        let result = job_enum(&all_jobs);
        assert_item_eq(&result, "schema/job/job_enum.rs");
    }
}
