use syn::parse_quote;

use crate::{
    JobConfig,
    utils::{dimension_ident, job_ident, variable_ident},
};

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
    use syn::parse_quote;

    use crate::configs::JobArg;

    use super::*;

    #[test]
    fn test_job_definition() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec![JobArg {
                id: "a".to_string(),
                over: vec![],
            }],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
        };
        let item = job_definition(&job);
        let expected: syn::ItemStruct = parse_quote! {
            #[doc = "A struct representing the job `beta`."]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub struct BetaJob {
                pub i: IDim,
            }
        };

        assert_eq!(item, expected);
    }

    #[test]
    fn test_job_definition_multiple_dims() {
        let job = JobConfig {
            id: "epsilon".to_string(),
            from: vec![
                JobArg {
                    id: "b".to_string(),
                    over: vec!["j".to_string()],
                },
                JobArg {
                    id: "d".to_string(),
                    over: vec!["j".to_string()],
                },
            ],
            to: "e".to_string(),
            dims: vec!["i".to_string(), "k".to_string()],
            spawn_dim: None,
        };
        let item = job_definition(&job);
        let expected: syn::ItemStruct = parse_quote! {
            #[doc = "A struct representing the job `epsilon`."]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub struct EpsilonJob {
                pub i: IDim,
                pub k: KDim,
            }
        };

        assert_eq!(item, expected);
    }
}
