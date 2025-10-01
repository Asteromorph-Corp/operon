use std::collections::HashMap;

use indexmap::IndexSet;

use crate::configs::EntityId;
use crate::{JobConfig, JobConfigMap};

/// Returns a mapping from job `to` entity to job `id`.
fn build_upstream_inverted_index(jobs: &JobConfigMap) -> HashMap<&EntityId, &JobConfig> {
    let mut index: HashMap<&String, &JobConfig> = HashMap::new();

    for job in jobs.values() {
        if let Some(dup) = index.get(&job.to) {
            panic!(
                "Duplicate entity: {} from {} and {}",
                job.to, dup.id, job.id
            );
        }

        index.insert(&job.to, job);
    }

    index
}

/// Return the full set of jobs that `target_job` depends on.
///
/// The returned jobs are sorted lexicographically by id.
pub fn get_upstream_jobs<'a>(
    target_job: &'a JobConfig,
    all_jobs: &'a JobConfigMap,
) -> IndexSet<&'a JobConfig> {
    let mut visited = IndexSet::new();
    let mut stack = vec![target_job];
    let inverted_index = build_upstream_inverted_index(all_jobs);

    // depth-first traversal
    while let Some(next) = stack.pop() {
        // skip if we've already processed this dependency
        if !visited.insert(next) {
            continue;
        }
        for entity in &next.from {
            let Some(dep) = inverted_index.get(&entity.id) else {
                continue; // input entity, no job produces it
            };

            if !visited.contains(dep) {
                stack.push(dep);
            }
        }
    }

    visited.sort_by(|a, b| a.id.cmp(&b.id));
    visited
}

/// Return the full set of jobs that `target_job` directly depends on.
///
/// The returned jobs are sorted lexicographically by id.
pub fn get_direct_upstream_jobs<'a>(
    target_job: &'a JobConfig,
    all_jobs: &'a JobConfigMap,
) -> IndexSet<&'a JobConfig> {
    let mut upstream_jobs = all_jobs
        .values()
        .filter(|job| target_job.from.iter().any(|dep| dep.id == job.to))
        .collect::<IndexSet<_>>();

    upstream_jobs.sort_by(|a, b| a.id.cmp(&b.id)); // Unstable sort because duplicates are not allowed in IndexSet

    upstream_jobs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JobArg;

    #[test]
    fn test_get_upstream_jobs() {
        let jobs = JobConfigMap::from_iter([
            (
                "beta".to_string(),
                JobConfig {
                    id: "beta".to_string(),
                    from: vec![JobArg {
                        id: "a".to_string(),
                        over: vec![],
                    }],
                    to: "b".to_string(),
                    dims: vec!["i".to_string()],
                    spawn_dim: Some("j".to_string()),
                    pool_size: 8,
                },
            ),
            (
                "gamma".to_string(),
                JobConfig {
                    id: "gamma".to_string(),
                    from: vec![JobArg {
                        id: "a".to_string(),
                        over: vec![],
                    }],
                    to: "c".to_string(),
                    dims: vec!["i".to_string()],
                    spawn_dim: Some("k".to_string()),
                    pool_size: 8,
                },
            ),
            (
                "delta".to_string(),
                JobConfig {
                    id: "delta".to_string(),
                    from: vec![
                        JobArg {
                            id: "a".to_string(),
                            over: vec![],
                        },
                        JobArg {
                            id: "b".to_string(),
                            over: vec![],
                        },
                        JobArg {
                            id: "c".to_string(),
                            over: vec![],
                        },
                    ],
                    to: "d".to_string(),
                    dims: vec!["i".to_string(), "j".to_string(), "k".to_string()],
                    spawn_dim: None,
                    pool_size: 4,
                },
            ),
            (
                "epsilon".to_string(),
                JobConfig {
                    id: "epsilon".to_string(),
                    from: vec![
                        JobArg {
                            id: "b".to_string(),
                            over: vec!["j".to_string()],
                        },
                        JobArg {
                            id: "d".to_string(),
                            over: vec!["j".to_string()],
                        },
                    ],
                    to: "e".to_string(),
                    dims: vec!["i".to_string(), "k".to_string()],
                    spawn_dim: None,
                    pool_size: 4,
                },
            ),
            (
                "zeta".to_string(),
                JobConfig {
                    id: "zeta".to_string(),
                    from: vec![
                        JobArg {
                            id: "c".to_string(),
                            over: vec!["k".to_string()],
                        },
                        JobArg {
                            id: "e".to_string(),
                            over: vec!["k".to_string()],
                        },
                    ],
                    to: "f".to_string(),
                    dims: vec!["i".to_string()],
                    spawn_dim: None,
                    pool_size: 1,
                },
            ),
        ]);

        let beta = jobs.get("beta").unwrap();
        let gamma = jobs.get("gamma").unwrap();
        let delta = jobs.get("delta").unwrap();
        let epsilon = jobs.get("epsilon").unwrap();
        let zeta = jobs.get("zeta").unwrap();

        assert_eq!(get_upstream_jobs(beta, &jobs), IndexSet::from([beta]));
        assert_eq!(get_upstream_jobs(gamma, &jobs), IndexSet::from([gamma]));
        assert_eq!(
            get_upstream_jobs(delta, &jobs),
            IndexSet::from([beta, gamma, delta])
        );
        assert_eq!(
            get_upstream_jobs(epsilon, &jobs),
            IndexSet::from([beta, gamma, delta, epsilon])
        );
        assert_eq!(
            get_upstream_jobs(zeta, &jobs),
            IndexSet::from([beta, gamma, delta, epsilon, zeta])
        );
    }

    #[test]
    fn test_get_direct_upstream_jobs() {
        let jobs = JobConfigMap::from_iter([
            (
                "beta".to_string(),
                JobConfig {
                    id: "beta".to_string(),
                    from: vec![JobArg {
                        id: "a".to_string(),
                        over: vec![],
                    }],
                    to: "b".to_string(),
                    dims: vec!["i".to_string()],
                    spawn_dim: Some("j".to_string()),
                    pool_size: 8,
                },
            ),
            (
                "gamma".to_string(),
                JobConfig {
                    id: "gamma".to_string(),
                    from: vec![JobArg {
                        id: "a".to_string(),
                        over: vec![],
                    }],
                    to: "c".to_string(),
                    dims: vec!["i".to_string()],
                    spawn_dim: Some("k".to_string()),
                    pool_size: 8,
                },
            ),
            (
                "delta".to_string(),
                JobConfig {
                    id: "delta".to_string(),
                    from: vec![
                        JobArg {
                            id: "a".to_string(),
                            over: vec![],
                        },
                        JobArg {
                            id: "b".to_string(),
                            over: vec![],
                        },
                        JobArg {
                            id: "c".to_string(),
                            over: vec![],
                        },
                    ],
                    to: "d".to_string(),
                    dims: vec!["i".to_string(), "j".to_string(), "k".to_string()],
                    spawn_dim: None,
                    pool_size: 4,
                },
            ),
            (
                "epsilon".to_string(),
                JobConfig {
                    id: "epsilon".to_string(),
                    from: vec![
                        JobArg {
                            id: "b".to_string(),
                            over: vec!["j".to_string()],
                        },
                        JobArg {
                            id: "d".to_string(),
                            over: vec!["j".to_string()],
                        },
                    ],
                    to: "e".to_string(),
                    dims: vec!["i".to_string(), "k".to_string()],
                    spawn_dim: None,
                    pool_size: 4,
                },
            ),
            (
                "zeta".to_string(),
                JobConfig {
                    id: "zeta".to_string(),
                    from: vec![
                        JobArg {
                            id: "c".to_string(),
                            over: vec!["k".to_string()],
                        },
                        JobArg {
                            id: "e".to_string(),
                            over: vec!["k".to_string()],
                        },
                    ],
                    to: "f".to_string(),
                    dims: vec!["i".to_string()],
                    spawn_dim: None,
                    pool_size: 1,
                },
            ),
        ]);

        let beta = jobs.get("beta").unwrap();
        let gamma = jobs.get("gamma").unwrap();
        let delta = jobs.get("delta").unwrap();
        let epsilon = jobs.get("epsilon").unwrap();
        let zeta = jobs.get("zeta").unwrap();

        assert_eq!(get_direct_upstream_jobs(beta, &jobs), IndexSet::from([]));
        assert_eq!(get_direct_upstream_jobs(gamma, &jobs), IndexSet::from([]));
        assert_eq!(
            get_direct_upstream_jobs(delta, &jobs),
            IndexSet::from([beta, gamma])
        );
        assert_eq!(
            get_direct_upstream_jobs(epsilon, &jobs),
            IndexSet::from([beta, delta])
        );
        assert_eq!(
            get_direct_upstream_jobs(zeta, &jobs),
            IndexSet::from([gamma, epsilon])
        );
    }
}
