use syn::parse_quote;

use crate::configs::JobConfigMap;
use crate::operon_ident;
use crate::utils::{job_enum_ident, variant_ident};

/// Generates an enum representing all jobs in the job configuration map.
///
/// # Example
/// ```rust,ignore
/// #[derive(Debug, Clone)]
/// pub enum JobEnum {
///     Alpha(operon::schema::Job<0usize>),
///     Beta(operon::schema::Job<1usize>),
///     Gamma(operon::schema::Job<1usize>),
///     Delta(operon::schema::Job<3usize>),
///     Epsilon(operon::schema::Job<2usize>),
///     Zeta(operon::schema::Job<1usize>),
/// }
/// ```
pub fn job_enum_definition(jobs: &JobConfigMap) -> syn::ItemEnum {
    let operon = operon_ident();
    let job_enum_ident = job_enum_ident();
    let variants = jobs.values().map(|job| -> syn::Variant {
        let variant_ident = variant_ident(&job.id);
        let n = job.dims.len();
        parse_quote! {
            #variant_ident(#operon::schema::Job<#n>)
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
    fn test_job_enum_definition(all_jobs: JobConfigMap) {
        let result = job_enum_definition(&all_jobs);
        assert_item_eq(&result, "schema/job/job_enum_definition.rs");
    }
}
