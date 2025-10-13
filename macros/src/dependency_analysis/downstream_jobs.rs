use indexmap::IndexSet;

use crate::configs::{JobConfig, JobConfigMap};

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
    use rstest::rstest;

    use super::*;
    use crate::test_utils::simple_pipeline::all_jobs;

    #[rstest]
    #[case::beta("beta", vec!["delta", "epsilon"])]
    #[case::gamma("gamma", vec!["delta", "zeta"])]
    #[case::delta("delta", vec!["epsilon"])]
    #[case::epsilon("epsilon", vec!["zeta"])]
    #[case::zeta("zeta", vec![])]
    fn test_get_direct_downstream_jobs(
        all_jobs: JobConfigMap,
        #[case] job_id: &str,
        #[case] expected_job_ids: Vec<&str>,
    ) {
        let job = all_jobs.get(job_id).unwrap();
        let expected = expected_job_ids
            .into_iter()
            .map(|id| all_jobs.get(id).unwrap())
            .collect::<IndexSet<_>>();
        assert_eq!(get_direct_downstream_jobs(job, &all_jobs), expected);
    }
}
