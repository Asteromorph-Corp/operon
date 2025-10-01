use indexmap::IndexSet;

use crate::configs::DimensionId;
use crate::{JobConfig, JobConfigMap};

/// Returns a set of job ids that are repeating jobs for the given target dimension.
pub fn get_jobs_repeating_on<'a>(
    target_dim: &'a DimensionId,
    all_jobs: &'a JobConfigMap,
) -> IndexSet<&'a JobConfig> {
    all_jobs
        .values()
        .filter(|job| job.dims.contains(target_dim))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JobArg;

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
