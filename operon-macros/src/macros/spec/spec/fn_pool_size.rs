use syn::parse_quote;

use crate::configs::{PoolSizeSpec, TaskConfig};

/// Generates the `pool_size` function for a task.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/spec/spec/fn_pool_size.rs") )]
/// ```
pub(super) fn fn_pool_size(task: &TaskConfig) -> syn::ImplItemFn {
    match &task.pool_size {
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
    use crate::test_utils::simple_pipeline::task_beta;

    fn task_beta_env() -> TaskConfig {
        TaskConfig {
            pool_size: PoolSizeSpec::Env("BETA_WORKERS".to_string()),
            ..task_beta()
        }
    }

    #[rstest]
    #[case::simple(task_beta(), "spec/spec/fn_pool_size.rs")]
    #[case::env(task_beta_env(), "spec/spec/fn_pool_size.env.rs")]
    fn test_fn_pool_size(#[case] task: TaskConfig, #[case] fixture_path: &str) {
        let item = fn_pool_size(&task);
        assert_item_eq(&item, fixture_path);
    }
}
