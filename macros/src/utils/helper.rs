use std::collections::HashMap;

use indexmap::IndexSet;

use crate::configs::{EntityId, JobConfigMap, JobId};

/// Returns a mapping from job `to` entity to job `id`.
fn build_upstream_inverted_index(jobs: &JobConfigMap) -> HashMap<&EntityId, &JobId> {
    let mut index: HashMap<&String, &String> = HashMap::new();

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
pub fn get_upstream_jobs<'a>(target_id: &'a JobId, jobs: &'a JobConfigMap) -> IndexSet<&'a JobId> {
    let mut visited = IndexSet::new();
    let mut stack = vec![target_id];
    let inverted_index = build_upstream_inverted_index(jobs);

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

fn build_downstream_inverted_index(jobs: &JobConfigMap) -> HashMap<&EntityId, Vec<&JobId>> {
    let mut index: HashMap<&String, Vec<&String>> = HashMap::new();

    for job in jobs.values() {
        for dep in &job.from {
            index.entry(dep).or_default().push(&job.id);
        }
    }

    index
}

/// Return the full set of job ids that depend on `target_id`.
///
/// The order of the returned ids is sorted lexicographically.
pub fn get_downstream_jobs<'a>(
    target_id: &'a JobId,
    jobs: &'a JobConfigMap,
) -> IndexSet<&'a JobId> {
    let mut visited = IndexSet::new();
    let mut stack = vec![target_id];
    let inverted_index = build_downstream_inverted_index(jobs);

    // depth-first traversal
    while let Some(next_id) = stack.pop() {
        // skip if we've already processed this job
        if !visited.insert(next_id) {
            continue;
        }

        // push *its* dependents onto the stack
        let Some(next_job) = jobs.get(next_id) else {
            continue;
        };

        if let Some(deps) = inverted_index.get(&next_job.to) {
            for dep_job in deps {
                if !visited.contains(dep_job) {
                    stack.push(dep_job);
                }
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
    fn test_get_upstream_jobs() {
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
            (
                "gamma".to_string(),
                JobConfig {
                    id: "gamma".to_string(),
                    from: vec!["a".to_string()],
                    to: "c".to_string(),
                    dims: vec!["i".to_string()],
                    spawn_dim: Some("k".to_string()),
                },
            ),
            (
                "delta".to_string(),
                JobConfig {
                    id: "delta".to_string(),
                    from: vec!["a".to_string(), "b".to_string(), "c".to_string()],
                    to: "d".to_string(),
                    dims: vec!["i".to_string(), "j".to_string(), "k".to_string()],
                    spawn_dim: None,
                },
            ),
            (
                "epsilon".to_string(),
                JobConfig {
                    id: "epsilon".to_string(),
                    from: vec!["b".to_string(), "d".to_string()],
                    to: "e".to_string(),
                    dims: vec!["i".to_string(), "k".to_string()],
                    spawn_dim: None,
                },
            ),
            (
                "zeta".to_string(),
                JobConfig {
                    id: "zeta".to_string(),
                    from: vec!["c".to_string(), "e".to_string()],
                    to: "f".to_string(),
                    dims: vec!["i".to_string()],
                    spawn_dim: None,
                },
            ),
        ]);

        assert_eq!(
            get_upstream_jobs(&"beta".to_string(), &jobs),
            IndexSet::from([&"beta".to_string()])
        );

        assert_eq!(
            get_upstream_jobs(&"gamma".to_string(), &jobs),
            IndexSet::from([&"gamma".to_string()])
        );

        assert_eq!(
            get_upstream_jobs(&"delta".to_string(), &jobs),
            IndexSet::from([
                &"beta".to_string(),
                &"gamma".to_string(),
                &"delta".to_string(),
            ])
        );

        assert_eq!(
            get_upstream_jobs(&"epsilon".to_string(), &jobs),
            IndexSet::from([
                &"beta".to_string(),
                &"gamma".to_string(),
                &"delta".to_string(),
                &"epsilon".to_string(),
            ])
        );

        assert_eq!(
            get_upstream_jobs(&"zeta".to_string(), &jobs),
            IndexSet::from([
                &"beta".to_string(),
                &"gamma".to_string(),
                &"delta".to_string(),
                &"epsilon".to_string(),
                &"zeta".to_string(),
            ])
        );
    }

    #[test]
    fn test_get_downstream_jobs() {
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
            (
                "gamma".to_string(),
                JobConfig {
                    id: "gamma".to_string(),
                    from: vec!["a".to_string()],
                    to: "c".to_string(),
                    dims: vec!["i".to_string()],
                    spawn_dim: Some("k".to_string()),
                },
            ),
            (
                "delta".to_string(),
                JobConfig {
                    id: "delta".to_string(),
                    from: vec!["a".to_string(), "b".to_string(), "c".to_string()],
                    to: "d".to_string(),
                    dims: vec!["i".to_string(), "j".to_string(), "k".to_string()],
                    spawn_dim: None,
                },
            ),
            (
                "epsilon".to_string(),
                JobConfig {
                    id: "epsilon".to_string(),
                    from: vec!["b".to_string(), "d".to_string()],
                    to: "e".to_string(),
                    dims: vec!["i".to_string(), "k".to_string()],
                    spawn_dim: None,
                },
            ),
            (
                "zeta".to_string(),
                JobConfig {
                    id: "zeta".to_string(),
                    from: vec!["c".to_string(), "e".to_string()],
                    to: "f".to_string(),
                    dims: vec!["i".to_string()],
                    spawn_dim: None,
                },
            ),
        ]);

        assert_eq!(
            get_downstream_jobs(&"beta".to_string(), &jobs),
            IndexSet::from([
                &"beta".to_string(),
                &"delta".to_string(),
                &"epsilon".to_string(),
                &"zeta".to_string(),
            ])
        );

        assert_eq!(
            get_downstream_jobs(&"gamma".to_string(), &jobs),
            IndexSet::from([
                &"gamma".to_string(),
                &"delta".to_string(),
                &"epsilon".to_string(),
                &"zeta".to_string(),
            ])
        );

        assert_eq!(
            get_downstream_jobs(&"delta".to_string(), &jobs),
            IndexSet::from([
                &"delta".to_string(),
                &"epsilon".to_string(),
                &"zeta".to_string(),
            ])
        );

        assert_eq!(
            get_downstream_jobs(&"epsilon".to_string(), &jobs),
            IndexSet::from([&"epsilon".to_string(), &"zeta".to_string()])
        );

        assert_eq!(
            get_downstream_jobs(&"zeta".to_string(), &jobs),
            IndexSet::from([&"zeta".to_string()])
        );
    }
}
