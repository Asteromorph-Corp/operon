use indexmap::IndexSet;

use crate::configs::{TaskConfig, TaskConfigMap};

/// Return the full set of tasks that directly depend on `target_task`.
///
/// The returned tasks are sorted lexicographically by id.
pub(crate) fn get_direct_downstream_tasks<'a>(
    target_task: &'a TaskConfig,
    all_tasks: &'a TaskConfigMap,
) -> IndexSet<&'a TaskConfig> {
    let mut downstream_tasks = all_tasks
        .values()
        .filter(|task| task.from.iter().any(|dep| dep.id == target_task.to))
        .collect::<IndexSet<_>>();

    downstream_tasks.sort_by(|a, b| a.id.cmp(&b.id)); // Unstable sort because duplicates are not allowed in IndexSet

    downstream_tasks
}

#[cfg(test)]
mod tests {
    use quote::format_ident;
    use rstest::rstest;

    use super::*;
    use crate::test_utils::simple_pipeline::all_tasks;

    #[rstest]
    #[case::beta(format_ident!("beta"), vec![format_ident!("delta"), format_ident!("epsilon")])]
    #[case::gamma(format_ident!("gamma"), vec![format_ident!("delta"), format_ident!("zeta")])]
    #[case::delta(format_ident!("delta"), vec![format_ident!("epsilon")])]
    #[case::epsilon(format_ident!("epsilon"), vec![format_ident!("zeta")])]
    #[case::zeta(format_ident!("zeta"), vec![])]
    fn test_get_direct_downstream_tasks(
        all_tasks: TaskConfigMap,
        #[case] task_id: syn::Ident,
        #[case] expected_task_ids: Vec<syn::Ident>,
    ) {
        let task = all_tasks.get(&task_id).unwrap();
        let expected = expected_task_ids
            .into_iter()
            .map(|id| all_tasks.get(&id).unwrap())
            .collect::<IndexSet<_>>();
        assert_eq!(get_direct_downstream_tasks(task, &all_tasks), expected);
    }
}
