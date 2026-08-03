use syn::parse_quote;

use crate::utils::spec_ident;

/// Generates a struct definition for a job specification.
///
/// # Example
/// ```rust,ignore
/// #[derive(Debug, Clone, Copy)]
/// pub struct BetaSpec;
/// ```
pub fn task_spec_definition(job_id: &syn::Ident) -> syn::ItemStruct {
    let spec_ident = spec_ident(job_id);

    parse_quote! {
        #[derive(Debug, Clone, Copy)]
        pub struct #spec_ident;
    }
}

#[cfg(test)]
mod tests {
    use quote::format_ident;
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;

    #[rstest]
    #[case::simple(format_ident!("beta"), "spec/spec/task_spec_definition.rs")]
    fn test_task_spec_definition(#[case] job_id: syn::Ident, #[case] fixture_path: &str) {
        let item = task_spec_definition(&job_id);
        assert_item_eq(&item, fixture_path);
    }
}
