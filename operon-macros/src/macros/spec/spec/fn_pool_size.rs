use syn::parse_quote;

use crate::configs::{JobConfig, PoolSizeSpec};

/// Generates the `pool_size` function for the implementation of the trait `TaskSpec`.
///
/// # Example (literal)
/// ```rust,ignore
/// fn pool_size(&self) -> usize {
///     8usize
/// }
/// ```
///
/// # Example (env var)
/// ```rust,ignore
/// fn pool_size(&self) -> usize {
///     let v = ::std::env::var("SOME_ENV")
///         .expect("Environment variable `SOME_ENV` not set for task concurrency")
///         .parse::<usize>()
///         .expect("Environment variable `SOME_ENV` is not a valid concurrency value");
///     assert!(v != 0, "Environment variable `SOME_ENV` must not be zero for task concurrency");
///     v
/// }
/// ```
pub(super) fn fn_pool_size(job: &JobConfig) -> syn::ImplItemFn {
    match &job.pool_size {
        PoolSizeSpec::Literal(n) => {
            let pool_size = *n;
            parse_quote! {
                fn pool_size(&self) -> usize {
                    #pool_size
                }
            }
        }
        PoolSizeSpec::Env(var_name) => {
            let expect_not_set =
                format!("Environment variable `{var_name}` not set for task concurrency");
            let expect_not_usize =
                format!("Environment variable `{var_name}` is not a valid concurrency value");
            let expect_nonzero =
                format!("Environment variable `{var_name}` must not be zero for task concurrency");
            parse_quote! {
                fn pool_size(&self) -> usize {
                    let v = ::std::env::var(#var_name)
                        .expect(#expect_not_set)
                        .parse::<usize>()
                        .expect(#expect_not_usize);
                    assert!(v != 0, #expect_nonzero);
                    v
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::configs::PoolSizeSpec;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::job_beta;

    fn job_beta_env() -> JobConfig {
        JobConfig {
            pool_size: PoolSizeSpec::Env("JOB_CONCURRENCY".to_string()),
            ..job_beta()
        }
    }

    #[rstest]
    #[case::simple(job_beta(), "spec/spec/fn_pool_size.rs")]
    #[case::env(job_beta_env(), "spec/spec/fn_pool_size.env.rs")]
    fn test_fn_pool_size(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let item = fn_pool_size(&job);
        assert_item_eq(&item, fixture_path);
    }
}
