use indexmap::IndexSet;

use crate::configs::{TaskConfig, TaskConfigMap};

/// Returns a set of task ids that are repeating tasks for the given target dimension.
pub(crate) fn get_tasks_repeating_on<'a>(
    target_dim: &'a syn::Ident,
    all_tasks: &'a TaskConfigMap,
) -> IndexSet<&'a TaskConfig> {
    all_tasks
        .values()
        .filter(|task| task.dims.contains(target_dim))
        .collect()
}

#[cfg(test)]
mod tests {
    use quote::format_ident;
    use rstest::rstest;

    use super::*;
    use crate::test_utils::simple_pipeline::all_tasks;

    #[rstest]
    #[case::i(format_ident!("i"), vec![format_ident!("beta"), format_ident!("gamma"), format_ident!("delta"), format_ident!("epsilon"), format_ident!("zeta")])]
    #[case::j(format_ident!("j"), vec![format_ident!("delta")])]
    #[case::k(format_ident!("k"), vec![format_ident!("delta"), format_ident!("epsilon")])]
    fn test_get_tasks_repeating_on(
        all_tasks: TaskConfigMap,
        #[case] dim_id: syn::Ident,
        #[case] expected_task_ids: Vec<syn::Ident>,
    ) {
        let expected = expected_task_ids
            .into_iter()
            .map(|id| all_tasks.get(&id).unwrap())
            .collect::<IndexSet<_>>();
        assert_eq!(get_tasks_repeating_on(&dim_id, &all_tasks), expected);
    }
}
