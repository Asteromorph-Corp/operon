use std::collections::HashMap;

use indexmap::IndexSet;

use crate::{
    JobConfig,
    configs::{DimensionId, EntityId, JobConfigMap},
};

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

/// Return the full set of job ids that `target_id` depends on.
///
/// The order of the returned ids is sorted lexicographically.
pub fn get_upstream_jobs<'a>(
    target_job: &'a JobConfig,
    jobs: &'a JobConfigMap,
) -> IndexSet<&'a JobConfig> {
    let mut visited = IndexSet::new();
    let mut stack = vec![target_job];
    let inverted_index = build_upstream_inverted_index(jobs);

    // depth-first traversal
    while let Some(next) = stack.pop() {
        // skip if we've already processed this dependency
        if !visited.insert(next) {
            continue;
        }

        for entity in &next.from {
            let Some(dep) = inverted_index.get(&entity.id) else {
                continue;
            };

            if !visited.contains(dep) {
                stack.push(dep);
            }
        }
    }

    visited.sort_by(|a, b| a.id.cmp(&b.id));

    visited
}

fn build_downstream_inverted_index(jobs: &JobConfigMap) -> HashMap<&EntityId, Vec<&JobConfig>> {
    let mut index: HashMap<&String, Vec<&JobConfig>> = HashMap::new();

    for job in jobs.values() {
        for dep in &job.from {
            index.entry(&dep.id).or_default().push(job);
        }
    }

    index
}

/// Return the full set of job ids that depend on `target_id`.
///
/// The order of the returned ids is sorted lexicographically.
pub fn get_downstream_jobs<'a>(
    target_job: &'a JobConfig,
    jobs: &'a JobConfigMap,
) -> IndexSet<&'a JobConfig> {
    let mut visited = IndexSet::new();
    let mut stack = vec![target_job];
    let inverted_index = build_downstream_inverted_index(jobs);

    // depth-first traversal
    while let Some(next) = stack.pop() {
        // skip if we've already processed this job
        if !visited.insert(next) {
            continue;
        }

        if let Some(deps) = inverted_index.get(&next.to) {
            for dep_job in deps {
                if !visited.contains(dep_job) {
                    stack.push(dep_job);
                }
            }
        }
    }

    visited.sort_by(|a, b| a.id.cmp(&b.id)); // Unstable sort because duplicates are not allowed in IndexSet

    visited
}

/// Returns a set of job ids that are repeating jobs for the given target dimension.
pub fn get_jobs_repeating_on<'a>(
    target_dim: &'a DimensionId,
    jobs: &'a JobConfigMap,
) -> IndexSet<&'a JobConfig> {
    jobs.values()
        .filter(|job| job.dims.contains(target_dim))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{JobConfig, configs::JobArg};

    use super::*;

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
    fn test_get_downstream_jobs() {
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
                },
            ),
        ]);

        let beta = jobs.get("beta").unwrap();
        let gamma = jobs.get("gamma").unwrap();
        let delta = jobs.get("delta").unwrap();
        let epsilon = jobs.get("epsilon").unwrap();
        let zeta = jobs.get("zeta").unwrap();

        assert_eq!(
            get_downstream_jobs(beta, &jobs),
            IndexSet::from([beta, delta, epsilon, zeta])
        );
        assert_eq!(
            get_downstream_jobs(gamma, &jobs),
            IndexSet::from([gamma, delta, epsilon, zeta])
        );
        assert_eq!(
            get_downstream_jobs(delta, &jobs),
            IndexSet::from([delta, epsilon, zeta])
        );
        assert_eq!(
            get_downstream_jobs(epsilon, &jobs),
            IndexSet::from([epsilon, zeta])
        );
        assert_eq!(get_downstream_jobs(zeta, &jobs), IndexSet::from([zeta]));
    }

    #[test]
    fn test_get_jobs_repeating_on() {
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
                },
            ),
        ]);

        let beta = jobs.get("beta").unwrap();
        let gamma = jobs.get("gamma").unwrap();
        let delta = jobs.get("delta").unwrap();
        let epsilon = jobs.get("epsilon").unwrap();
        let zeta = jobs.get("zeta").unwrap();

        let i_dim = DimensionId::from("i");
        let j_dim = DimensionId::from("j");
        let k_dim = DimensionId::from("k");

        assert_eq!(
            get_jobs_repeating_on(&i_dim, &jobs),
            IndexSet::from([beta, gamma, delta, epsilon, zeta])
        );
        assert_eq!(
            get_jobs_repeating_on(&j_dim, &jobs),
            IndexSet::from([delta])
        );
        assert_eq!(
            get_jobs_repeating_on(&k_dim, &jobs),
            IndexSet::from([delta, epsilon])
        );
    }
}
