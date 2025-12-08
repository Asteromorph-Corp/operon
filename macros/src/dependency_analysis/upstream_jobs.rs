use std::collections::HashMap;

use indexmap::IndexSet;

use crate::configs::{JobConfig, JobConfigMap};

/// Returns a mapping from job `to` entity to job `id`.
fn build_upstream_inverted_index(jobs: &JobConfigMap) -> HashMap<&syn::Ident, &JobConfig> {
    let mut index: HashMap<&syn::Ident, &JobConfig> = HashMap::new();

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
    use rstest::rstest;

    use super::*;
    use crate::configs::JobConfigMap;
    use crate::test_utils::simple_pipeline::all_jobs;

    #[rstest]
    #[case::beta("beta", vec!["alpha", "beta"])]
    #[case::gamma("gamma", vec!["alpha", "gamma"])]
    #[case::delta("delta", vec!["alpha", "beta", "gamma", "delta"])]
    #[case::epsilon("epsilon", vec!["alpha", "beta", "gamma", "delta", "epsilon"])]
    #[case::zeta("zeta", vec!["alpha", "beta", "gamma", "delta", "epsilon", "zeta"])]
    fn test_get_upstream_jobs(
        all_jobs: JobConfigMap,
        #[case] job_id: &str,
        #[case] expected_job_ids: Vec<&str>,
    ) {
        let job = all_jobs.get(job_id).unwrap();
        let expected = expected_job_ids
            .into_iter()
            .map(|id| all_jobs.get(id).unwrap())
            .collect::<IndexSet<_>>();
        assert_eq!(get_upstream_jobs(job, &all_jobs), expected);
    }

    #[rstest]
    #[case::beta("beta", vec!["alpha"])]
    #[case::gamma("gamma", vec!["alpha"])]
    #[case::delta("delta", vec!["alpha", "beta", "gamma"])]
    #[case::epsilon("epsilon", vec!["beta", "delta"])]
    #[case::zeta("zeta", vec!["gamma", "epsilon"])]
    fn test_get_direct_upstream_jobs(
        all_jobs: JobConfigMap,
        #[case] job_id: &str,
        #[case] expected_job_ids: Vec<&str>,
    ) {
        let job = all_jobs.get(job_id).unwrap();
        let expected = expected_job_ids
            .into_iter()
            .map(|id| all_jobs.get(id).unwrap())
            .collect::<IndexSet<_>>();
        assert_eq!(get_direct_upstream_jobs(job, &all_jobs), expected);
    }
}
