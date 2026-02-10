use syn::parse_quote;

use crate::configs::JobConfig;

/// Generates the `pool_size` function for the implementation of the trait `JobSpec`.
///
/// # Example
/// ```rust,ignore
/// fn pool_size(&self) -> usize {
///     8usize
/// }
/// ```
pub(super) fn fn_pool_size(job: &JobConfig) -> syn::ImplItemFn {
    let pool_size = job.pool_size;

    parse_quote! {
        fn pool_size(&self) -> usize {
            #pool_size
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::job_beta;

    #[rstest]
    #[case::simple(job_beta(), "spec/spec/fn_pool_size.rs")]
    fn test_fn_pool_size(#[case] job: JobConfig, #[case] fixture_path: &str) {
        let item = fn_pool_size(&job);
        assert_item_eq(&item, fixture_path);
    }
}
