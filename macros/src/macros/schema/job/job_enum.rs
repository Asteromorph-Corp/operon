use syn::parse_quote;

use crate::{
    configs::JobConfigMap,
    utils::{job_enum_ident, job_ident, variant_ident},
};

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
    use quote::ToTokens;
    use syn::parse_quote;

    use crate::JobConfig;

    use super::*;

    #[test]
    fn test_job_enum() {
        let jobs = JobConfigMap::from_iter([
            (
                "beta".to_string(),
                JobConfig {
                    id: "beta".to_string(),
                    from: vec!["a".to_string()],
                    to: "b".to_string(),
                    dims: vec!["i".to_string()],
                },
            ),
            (
                "gamma".to_string(),
                JobConfig {
                    id: "gamma".to_string(),
                    from: vec!["a".to_string()],
                    to: "c".to_string(),
                    dims: vec!["j".to_string()],
                },
            ),
        ]);

        let item = job_enum(&jobs);
        let expected: syn::ItemEnum = parse_quote! {
            #[doc = "An enum representing any job."]
            #[derive(Debug, Clone)]
            pub enum JobEnum {
                Beta(BetaJob),
                Gamma(GammaJob),
            }
        };
        assert_eq!(
            item.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
