use std::collections::HashMap;

use indexmap::IndexSet;

use crate::configs::{EntityId, JobConfigMap, JobId};

/// Returns a mapping from job `to` entity to job `id`.
fn build_inverted_index(jobs: &JobConfigMap) -> HashMap<&EntityId, &JobId> {
    let mut index = HashMap::new();

    for job in jobs.values() {
        if let Some(dup_id) = index.get(&job.to) {
            panic!(
                "Duplicate entity: {} from {} and {}",
                job.to, dup_id, job.id
            );
        }

        index.insert(&job.to, &job.id);
    }

    index
}

/// Return the full set of job ids that `target_id` depends on.
///
/// The order of the returned ids is sorted lexicographically.
pub fn find_dependencies<'a>(target_id: &'a JobId, jobs: &'a JobConfigMap) -> IndexSet<&'a JobId> {
    let mut visited = IndexSet::new();
    let mut stack = vec![target_id];
    let inverted_index = build_inverted_index(jobs);

    // depth-first traversal
    while let Some(next_id) = stack.pop() {
        // skip if we've already processed this dependency
        if !visited.insert(next_id) {
            continue;
        }

        // push *its* prerequisites onto the stack
        let Some(next_job) = jobs.get(next_id) else {
            continue;
        };

        for entity in &next_job.from {
            let Some(dep_job) = inverted_index.get(entity) else {
                continue;
            };

            if !visited.contains(dep_job) {
                stack.push(dep_job);
            }
        }
    }

    visited.sort_unstable(); // Unstable sort because duplicates are not allowed in IndexSet

    visited
}

#[cfg(test)]
mod tests {
    use crate::JobConfig;

    use super::*;

    #[test]
    fn test_dependencies() {
        let jobs = JobConfigMap::from_iter([
            (
                "beta".to_string(),
                JobConfig {
                    id: "beta".to_string(),
                    from: vec!["a".to_string()],
                    to: "b".to_string(),
                    dims: vec!["i".to_string(), "j".to_string()],
                },
            ),
            (
                "gamma".to_string(),
                JobConfig {
                    id: "gamma".to_string(),
                    from: vec!["a".to_string()],
                    to: "c".to_string(),
                    dims: vec!["i".to_string(), "j".to_string()],
                },
            ),
            (
                "delta".to_string(),
                JobConfig {
                    id: "delta".to_string(),
                    from: vec!["a".to_string(), "b".to_string(), "c".to_string()],
                    to: "d".to_string(),
                    dims: vec!["i".to_string(), "k".to_string()],
                },
            ),
            (
                "epsilon".to_string(),
                JobConfig {
                    id: "epsilon".to_string(),
                    from: vec!["b".to_string(), "d".to_string()],
                    to: "e".to_string(),
                    dims: vec![],
                },
            ),
            (
                "zeta".to_string(),
                JobConfig {
                    id: "zeta".to_string(),
                    from: vec!["c".to_string(), "e".to_string()],
                    to: "f".to_string(),
                    dims: vec!["i".to_string()],
                },
            ),
        ]);

        assert_eq!(
            find_dependencies(&"beta".to_string(), &jobs),
            IndexSet::from([&"beta".to_string()])
        );

        assert_eq!(
            find_dependencies(&"gamma".to_string(), &jobs),
            IndexSet::from([&"gamma".to_string()])
        );

        assert_eq!(
            find_dependencies(&"delta".to_string(), &jobs),
            IndexSet::from([
                &"beta".to_string(),
                &"gamma".to_string(),
                &"delta".to_string()
            ])
        );

        assert_eq!(
            find_dependencies(&"epsilon".to_string(), &jobs),
            IndexSet::from([
                &"beta".to_string(),
                &"gamma".to_string(),
                &"delta".to_string(),
                &"epsilon".to_string()
            ])
        );

        assert_eq!(
            find_dependencies(&"zeta".to_string(), &jobs),
            IndexSet::from([
                &"beta".to_string(),
                &"gamma".to_string(),
                &"delta".to_string(),
                &"epsilon".to_string(),
                &"zeta".to_string()
            ])
        );
    }
}
