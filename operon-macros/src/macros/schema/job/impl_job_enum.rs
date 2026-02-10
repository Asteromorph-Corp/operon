use syn::parse_quote;

use crate::utils::{job_enum_ident, operon_ident};

/// Generates an implementation of the `JobEnum` trait for the `JobEnum` type.
///
/// # Example
/// ```rust,ignore
/// #[automatically_derived]
/// impl operon::schema::JobEnum for JobEnum {}
/// ```
pub fn impl_job_enum() -> syn::ItemImpl {
    let operon = operon_ident();
    let job_enum_ident = job_enum_ident();

    parse_quote! {
        #[automatically_derived]
        impl #operon::schema::JobEnum for #job_enum_ident {}
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;

    #[rstest]
    fn test_impl_job_enum() {
        let result = impl_job_enum();
        assert_item_eq(&result, "schema/job/impl_job_enum.rs");
    }
}
