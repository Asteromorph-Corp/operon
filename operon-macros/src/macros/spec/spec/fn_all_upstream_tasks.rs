use indexmap::IndexSet;
use syn::parse_quote;

use crate::configs::TaskConfig;
use crate::utils::to_lit_str;

pub fn fn_all_upstream_tasks(all_upstream_tasks: &IndexSet<&TaskConfig>) -> syn::ImplItemFn {
    let all_upstream_task_ids = all_upstream_tasks.iter().map(|j| to_lit_str(&j.id));

    parse_quote! {
        fn all_upstream_tasks(&self) -> Vec<&'static str> {
            vec![#(#all_upstream_task_ids),*]
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::configs::TaskConfigMap;
    use crate::dependency_analysis::get_upstream_tasks;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_tasks, task_epsilon};

    #[rstest]
    #[case::simple(task_epsilon(), "spec/spec/fn_all_upstream_tasks.rs")]
    fn test_fn_all_upstream_tasks(
        all_tasks: TaskConfigMap,
        #[case] task: TaskConfig,
        #[case] fixture_path: &str,
    ) {
        let upstream_tasks = get_upstream_tasks(&task, &all_tasks);
        let item = fn_all_upstream_tasks(&upstream_tasks);
        assert_item_eq(&item, fixture_path)
    }
}
