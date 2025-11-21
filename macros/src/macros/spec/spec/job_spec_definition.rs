use syn::parse_quote;

use crate::configs::JobId;
use crate::utils::spec_ident;

/// Generates a struct definition for a job specification.
///
/// Example:
/// ```rust,ignore
/// #[derive(Debug, Clone, Copy)]
/// pub struct BetaSpec;
/// ```
pub fn job_spec_definition(job_id: &JobId) -> syn::ItemStruct {
    let spec_ident = spec_ident(job_id);

    parse_quote! {
        #[derive(Debug, Clone, Copy)]
        pub struct #spec_ident;
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;

    #[rstest]
    #[case::simple("beta", "spec/job_spec_definition.rs")]
    fn test_job_spec_definition(#[case] job_id: &str, #[case] fixture_path: &str) {
        let job_id = JobId::from(job_id);
        let item = job_spec_definition(&job_id);
        assert_item_eq(&item, fixture_path);
    }
}
