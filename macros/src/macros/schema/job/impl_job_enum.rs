use syn::parse_quote;

use crate::utils::{job_enum_ident, operon_ident};

pub(super) fn impl_job_enum() -> syn::ItemImpl {
    let operon = operon_ident();
    let job_enum_ident = job_enum_ident();

    parse_quote! {
        impl #operon::schema_base::JobEnum for #job_enum_ident {}
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::*;

    #[test]
    fn test_impl_job_enum() {
        let result = impl_job_enum();
        let expected: syn::ItemImpl = parse_quote! {
            impl operon::schema_base::JobEnum for JobEnum {}
        };
        assert_eq!(
            result.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
