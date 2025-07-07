use syn::parse_quote;

use crate::{
    JobConfig,
    configs::JobConfigMap,
    utils::{find_dependencies, job_id_ident, job_ident},
};

/// Generates an implementation of the `Job` trait for the given job configuration.
///
/// Example:
/// ```rust,ignore
/// #[automatically_derived]
/// impl operon::schema_base::Job for BetaJob {
///     fn id() -> &'static str {
///         BETA_ID
///     }
///
///     fn is_descendant_of(other: &str) -> bool {
///         other == BETA_ID
///     }
/// }
/// ```
pub(super) fn impl_job(job: &JobConfig, jobs: &JobConfigMap) -> syn::ItemImpl {
    let operon = crate::utils::operon_ident();
    let job_ident = job_ident(&job.id);

    let id_ident = job_id_ident(&job.id);
    let deps = find_dependencies(&job.id, jobs)
        .into_iter()
        .map(job_id_ident);

    parse_quote! {
        #[automatically_derived]
        impl #operon::schema_base::Job for #job_ident {
            fn id() -> &'static str {
                #id_ident
            }

            fn is_descendant_of(other: &str) -> bool {
                #(
                    other == #deps
                )||*
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::parse_quote;

    use crate::{JobConfig, configs::JobConfigMap};

    use super::*;

    #[test]
    fn test_impl_job() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec!["a".to_string()],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
        };
        let jobs = JobConfigMap::from_iter([("beta".to_string(), job.clone())]);

        let item = impl_job(&job, &jobs);
        let expected: syn::ItemImpl = parse_quote! {
            #[automatically_derived]
            impl operon::schema_base::Job for BetaJob {
                fn id() -> &'static str {
                    BETA_ID
                }

                fn is_descendant_of(other: &str) -> bool {
                    other == BETA_ID
                }
            }
        };

        assert_eq!(
            item.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }

    #[test]
    fn test_impl_job_with_dependencies() {
        let job_epsilon = JobConfig {
            id: "epsilon".to_string(),
            from: vec!["b".to_string(), "d".to_string()],
            to: "e".to_string(),
            dims: vec!["i".to_string(), "k".to_string()],
            spawn_dim: None,
        };

        let jobs = JobConfigMap::from_iter([
            (
                "beta".to_string(),
                JobConfig {
                    id: "beta".to_string(),
                    from: vec!["a".to_string()],
                    to: "b".to_string(),
                    dims: vec!["i".to_string()],
                    spawn_dim: Some("j".to_string()),
                },
            ),
            ("epsilon".to_string(), job_epsilon.clone()),
        ]);

        let item = impl_job(&job_epsilon, &jobs);
        let expected: syn::ItemImpl = parse_quote! {
            #[automatically_derived]
            impl operon::schema_base::Job for EpsilonJob {
                fn id() -> &'static str {
                    EPSILON_ID
                }

                fn is_descendant_of(other: &str) -> bool {
                    other == BETA_ID || other == EPSILON_ID
                }
            }
        };

        assert_eq!(
            item.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
