use syn::parse_quote;

use crate::{
    JobConfig,
    utils::{job_enum_ident, job_ident, variant_ident},
};

pub(super) fn impl_enum_from_job(job: &JobConfig) -> syn::ItemImpl {
    let job_enum_ident = job_enum_ident();
    let job_ident = job_ident(&job.id);
    let variant_ident = variant_ident(&job.id);

    parse_quote! {
        impl From<#job_ident> for #job_enum_ident {
            fn from(job: #job_ident) -> Self {
                #job_enum_ident::#variant_ident(job)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_impl_enum_from_job() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec!["a".to_string()],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
        };
        let item = impl_enum_from_job(&job);
        let expected: syn::ItemImpl = parse_quote! {
            impl From<BetaJob> for JobEnum {
                fn from(job: BetaJob) -> Self {
                    JobEnum::Beta(job)
                }
            }
        };
        assert_eq!(
            item.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
