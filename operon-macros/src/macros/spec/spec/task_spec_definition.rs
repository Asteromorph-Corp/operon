use syn::parse_quote;

use crate::utils::spec_ident;

/// Generates a spec struct for a task.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/spec/spec/task_spec_definition.rs"))]
/// ```
pub fn task_spec_definition(task_id: &syn::Ident) -> syn::ItemStruct {
    let spec_ident = spec_ident(task_id);

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
    fn test_task_spec_definition(#[case] task_id: syn::Ident, #[case] fixture_path: &str) {
        let item = task_spec_definition(&task_id);
        assert_item_eq(&item, fixture_path);
    }
}
