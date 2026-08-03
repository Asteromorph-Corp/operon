use indexmap::IndexSet;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::to_lit_str;

pub fn fn_all_upstream_tasks(all_upstream_tasks: &IndexSet<&JobConfig>) -> syn::ImplItemFn {
    let all_upstream_job_ids = all_upstream_tasks.iter().map(|j| to_lit_str(&j.id));

    parse_quote! {
        fn all_upstream_tasks(&self) -> Vec<&'static str> {
            vec![#(#all_upstream_job_ids),*]
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::configs::JobConfigMap;
    use crate::dependency_analysis::get_upstream_jobs;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_jobs, job_epsilon};

    #[rstest]
    #[case::simple(job_epsilon(), "spec/spec/fn_all_upstream_tasks.rs")]
    fn test_fn_all_upstream_tasks(
        all_jobs: JobConfigMap,
        #[case] job: JobConfig,
        #[case] fixture_path: &str,
    ) {
        let upstream_jobs = get_upstream_jobs(&job, &all_jobs);
        let item = fn_all_upstream_tasks(&upstream_jobs);
        assert_item_eq(&item, fixture_path)
    }
}
