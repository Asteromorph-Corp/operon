use syn::parse_quote;

use crate::{
    JobConfig,
    utils::{arg_ident, dimension_ident, job_ident},
};

pub(super) fn job_definition(job: &JobConfig) -> syn::ItemStruct {
    let job_ident = job_ident(&job.id);
    let dims = job.dims.iter().map(|dim| -> syn::Field {
        let arg = arg_ident(dim);
        let dim_ident = dimension_ident(dim);

        parse_quote! {
            pub #arg: schema::#dim_ident
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
    use quote::ToTokens;
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_job_definition() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec!["a".to_string()],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
        };
        let item = job_definition(&job);
        let expected: syn::ItemStruct = parse_quote! {
            #[doc = "A struct representing the job `beta`."]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub struct BetaJob {
                pub i: schema::I,
            }
        };

        assert_eq!(
            item.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }

    #[test]
    fn test_job_definition_multiple_dims() {
        let job = JobConfig {
            id: "gamma".to_string(),
            from: vec!["a".to_string()],
            to: "c".to_string(),
            dims: vec!["i".to_string(), "j".to_string()],
        };
        let item = job_definition(&job);
        let expected: syn::ItemStruct = parse_quote! {
            #[doc = "A struct representing the job `gamma`."]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub struct GammaJob {
                pub i: schema::I,
                pub j: schema::J,
            }
        };

        assert_eq!(
            item.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
