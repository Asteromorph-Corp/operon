use indexmap::IndexSet;

use crate::{JobConfig, JobConfigMap};

/// Return the full set of jobs that directly depend on `target_job`.
///
/// The returned jobs are sorted lexicographically by id.
pub fn get_direct_downstream_jobs<'a>(
    target_job: &'a JobConfig,
    all_jobs: &'a JobConfigMap,
) -> IndexSet<&'a JobConfig> {
    let mut downstream_jobs = all_jobs
        .values()
        .filter(|job| job.from.iter().any(|dep| dep.id == target_job.to))
        .collect::<IndexSet<_>>();

    downstream_jobs.sort_by(|a, b| a.id.cmp(&b.id)); // Unstable sort because duplicates are not allowed in IndexSet

    downstream_jobs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JobArg;

    #[test]
    fn test_get_direct_downstream_jobs() {
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

        assert_eq!(
            get_direct_downstream_jobs(beta, &jobs),
            IndexSet::from([delta, epsilon])
        );
        assert_eq!(
            get_direct_downstream_jobs(gamma, &jobs),
            IndexSet::from([delta, zeta])
        );
        assert_eq!(
            get_direct_downstream_jobs(delta, &jobs),
            IndexSet::from([epsilon])
        );
        assert_eq!(
            get_direct_downstream_jobs(epsilon, &jobs),
            IndexSet::from([zeta])
        );
        assert_eq!(get_direct_downstream_jobs(zeta, &jobs), IndexSet::from([]));
    }
}
