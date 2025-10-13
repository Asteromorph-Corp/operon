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
    use rstest::rstest;

    use super::*;
    use crate::test_utils::simple_pipeline::all_jobs;

    #[rstest]
    #[case::i("i", vec!["beta", "gamma", "delta", "epsilon", "zeta"])]
    #[case::j("j", vec!["delta"])]
    #[case::k("k", vec!["delta", "epsilon"])]
    fn test_get_jobs_repeating_on(
        all_jobs: JobConfigMap,
        #[case] dim_id: &str,
        #[case] expected_job_ids: Vec<&str>,
    ) {
        let dim_id = DimensionId::from(dim_id);
        let expected = expected_job_ids
            .into_iter()
            .map(|id| all_jobs.get(id).unwrap())
            .collect::<IndexSet<_>>();
        assert_eq!(get_jobs_repeating_on(&dim_id, &all_jobs), expected);
    }
}
