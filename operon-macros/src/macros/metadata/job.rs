use quote::quote;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::operon_ident;
use crate::utils::{job_metadata_ident, to_lit_str};

/// Generates a metadata function for a job.
///
/// # Example
/// ```rust,ignore
/// pub const fn job_beta_meta() -> operon::__private::JobMetadata<1usize> {
///     operon::__private::JobMetadata {
///         id: "beta",
///         dims: ["i"],
///         spawn_dim: Some("j"),
///         priority: &[("i", false)],
///     }
/// }
/// ```
pub fn job_metadata(job: &JobConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = job_metadata_ident(&job.id);
    let n = job.dims.len();
    let id = to_lit_str(&job.id);
    let dims = job.dims.iter().map(to_lit_str);
    let spawn_dim: syn::Expr = match &job.spawn_dim {
        Some(spawn_dim) => {
            let spawn_dim = to_lit_str(spawn_dim);
            parse_quote! { Some(#spawn_dim) }
        }
        None => parse_quote! { None },
    };
    let priority_items = job.priority.iter().map(|(dim, desc)| {
        let dim_str = to_lit_str(dim);
        quote! { (#dim_str, #desc) }
    });

    parse_quote! {
        pub const fn #fn_name() -> #operon::__private::JobMetadata<#n> {
            #operon::__private::JobMetadata {
                id: #id,
                dims: [#(#dims),*],
                spawn_dim: #spawn_dim,
                priority: &[#(#priority_items),*],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::format_ident;
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::job_beta;

    fn job_beta_with_priority() -> JobConfig {
        JobConfig {
            priority: vec![(format_ident!("i"), false)],
            ..job_beta()
        }
    }

    #[rstest]
    #[case(job_beta(), "metadata/job.rs")]
    #[case(job_beta_with_priority(), "metadata/job.with_priority.rs")]
    fn test_job_metadata(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let result = job_metadata(&job);
        assert_item_eq(&result, fixture_path);
    }
}
