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
    use super::*;

    #[test]
    fn test_job_spec_definition() {
        let job_id = JobId::from("beta");
        let result = job_spec_definition(&job_id);
        let expected: syn::ItemStruct = parse_quote! {
            #[derive(Debug, Clone, Copy)]
            pub struct BetaSpec;
        };

        assert_eq!(result, expected);
    }
}
