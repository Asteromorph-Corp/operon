use std::collections::HashMap;

use indexmap::IndexSet;

use crate::configs::{TaskConfig, TaskConfigMap};

/// Returns a mapping from task `to` entity to task `id`.
fn build_upstream_inverted_index(tasks: &TaskConfigMap) -> HashMap<&syn::Ident, &TaskConfig> {
    let mut index: HashMap<&syn::Ident, &TaskConfig> = HashMap::new();

    for task in tasks.values() {
        if let Some(dup) = index.get(&task.to) {
            panic!(
                "Duplicate entity: {} from {} and {}",
                task.to, dup.id, task.id
            );
        }

        let _ = index.insert(&task.to, task);
    }

    index
}

/// Return the full set of tasks that `target_task` depends on.
///
/// The returned tasks are sorted lexicographically by id.
pub(crate) fn get_upstream_tasks<'a>(
    target_task: &'a TaskConfig,
    all_tasks: &'a TaskConfigMap,
) -> IndexSet<&'a TaskConfig> {
    let mut visited = IndexSet::new();
    let mut stack = vec![target_task];
    let inverted_index = build_upstream_inverted_index(all_tasks);

    // depth-first traversal
    while let Some(next) = stack.pop() {
        // skip if we've already processed this dependency
        if !visited.insert(next) {
            continue;
        }
        for entity in &next.from {
            let Some(dep) = inverted_index.get(&entity.id) else {
                continue; // input entity, no task produces it
            };

            if !visited.contains(dep) {
                stack.push(dep);
            }
        }
    }

    visited.sort_by(|a, b| a.id.cmp(&b.id));
    visited
}

/// Return the full set of tasks that `target_task` directly depends on.
///
/// The returned tasks are sorted lexicographically by id.
pub(crate) fn get_direct_upstream_tasks<'a>(
    target_task: &'a TaskConfig,
    all_tasks: &'a TaskConfigMap,
) -> IndexSet<&'a TaskConfig> {
    let mut upstream_tasks = all_tasks
        .values()
        .filter(|task| target_task.from.iter().any(|dep| dep.id == task.to))
        .collect::<IndexSet<_>>();

    upstream_tasks.sort_by(|a, b| a.id.cmp(&b.id)); // Unstable sort because duplicates are not allowed in IndexSet

    upstream_tasks
}

#[cfg(test)]
mod tests {
    use quote::format_ident;
    use rstest::rstest;

    use super::*;
    use crate::configs::TaskConfigMap;
    use crate::test_utils::simple_pipeline::all_tasks;

    #[rstest]
    #[case::beta(format_ident!("beta"), vec![format_ident!("alpha"), format_ident!("beta")])]
    #[case::gamma(format_ident!("gamma"), vec![format_ident!("alpha"), format_ident!("gamma")])]
    #[case::delta(format_ident!("delta"), vec![format_ident!("alpha"), format_ident!("beta"), format_ident!("gamma"), format_ident!("delta")])]
    #[case::epsilon(format_ident!("epsilon"), vec![format_ident!("alpha"), format_ident!("beta"), format_ident!("gamma"), format_ident!("delta"), format_ident!("epsilon")])]
    #[case::zeta(format_ident!("zeta"), vec![format_ident!("alpha"), format_ident!("beta"), format_ident!("gamma"), format_ident!("delta"), format_ident!("epsilon"), format_ident!("zeta")])]
    fn test_get_upstream_tasks(
        all_tasks: TaskConfigMap,
        #[case] task_id: syn::Ident,
        #[case] expected_task_ids: Vec<syn::Ident>,
    ) {
        let task = all_tasks.get(&task_id).unwrap();
        let expected = expected_task_ids
            .into_iter()
            .map(|id| all_tasks.get(&id).unwrap())
            .collect::<IndexSet<_>>();
        assert_eq!(get_upstream_tasks(task, &all_tasks), expected);
    }

    #[rstest]
    #[case::beta(format_ident!("beta"), vec![format_ident!("alpha")])]
    #[case::gamma(format_ident!("gamma"), vec![format_ident!("alpha")])]
    #[case::delta(format_ident!("delta"), vec![format_ident!("alpha"), format_ident!("beta"), format_ident!("gamma")])]
    #[case::epsilon(format_ident!("epsilon"), vec![format_ident!("beta"), format_ident!("delta")])]
    #[case::zeta(format_ident!("zeta"), vec![format_ident!("gamma"), format_ident!("epsilon")])]
    fn test_get_direct_upstream_tasks(
        all_tasks: TaskConfigMap,
        #[case] task_id: syn::Ident,
        #[case] expected_task_ids: Vec<syn::Ident>,
    ) {
        let task = all_tasks.get(&task_id).unwrap();
        let expected = expected_task_ids
            .into_iter()
            .map(|id| all_tasks.get(&id).unwrap())
            .collect::<IndexSet<_>>();
        assert_eq!(get_direct_upstream_tasks(task, &all_tasks), expected);
    }
}
