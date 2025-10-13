use syn::parse_quote;

use crate::JobConfig;
use crate::utils::{dimension_ident, job_ident, variable_ident};

/// Generates a struct definition for the job, which includes fields for each dimension.
///
/// Example:
/// ```rust,ignore
/// #[doc = "A struct representing the job `beta`."]
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// pub struct BetaJob {
///     pub i: IDim,
/// }
/// ```
pub(super) fn job_definition(job: &JobConfig) -> syn::ItemStruct {
    let job_ident = job_ident(&job.id);
    let dims = job.dims.iter().map(|dim| -> syn::Field {
        let arg = variable_ident(dim);
        let dim_ident = dimension_ident(dim);

        parse_quote! {
            pub #arg: #dim_ident
        }
    });

    let doc = format!("A struct representing the job `{}`.", job.id);

    parse_quote! {
        #[doc = #doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct #job_ident {
            #(#dims,)*
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{job_beta, job_epsilon};

    #[rstest]
    #[case::simple(job_beta(), "schema/job/job_definition.simple.rs")]
    #[case::multiple_dims(job_epsilon(), "schema/job/job_definition.multiple_dims.rs")]
    fn test_job_definition(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let result = job_definition(&job);
        assert_item_eq(&result, fixture_path);
    }
}
