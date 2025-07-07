use syn::parse_quote;

use crate::{
    JobConfig,
    utils::{job_ident, rebuilder_ident, resolution_ident},
};

/// Generates a struct definition for a job rebuilder.
///
/// Example:
/// ```rust,ignore
/// #[derive(Debug)]
/// pub struct BetaRebuilder(Vec<(schema::BetaJob, schema::JResolution)>);
/// ```
pub fn job_rebuilder_definition(job: &JobConfig) -> syn::ItemStruct {
    let rebuilder_ident = rebuilder_ident(&job.id);
    let job_ident = job_ident(&job.id);
    let res_ident: syn::Type = match &job.spawn_dim {
        Some(dim) => {
            let res_ident = resolution_ident(dim);
            parse_quote! { schema::#res_ident }
        }
        None => parse_quote! { () },
    };

    parse_quote! {
        #[derive(Debug)]
        pub struct #rebuilder_ident(Vec<(schema::#job_ident, #res_ident)>);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_rebuilder_definition() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec!["a".to_string()],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
        };
        let result = job_rebuilder_definition(&job);
        let expected: syn::ItemStruct = parse_quote! {
            #[derive(Debug)]
            pub struct BetaRebuilder(Vec<(schema::BetaJob, schema::JResolution)>);
        };

        assert_eq!(result, expected);
    }
}
