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
    use quote::format_ident;
    use rstest::rstest;

    use super::*;
    use crate::test_utils::simple_pipeline::all_jobs;

    #[rstest]
    #[case::beta(format_ident!("beta"), vec![format_ident!("delta"), format_ident!("epsilon")])]
    #[case::gamma(format_ident!("gamma"), vec![format_ident!("delta"), format_ident!("zeta")])]
    #[case::delta(format_ident!("delta"), vec![format_ident!("epsilon")])]
    #[case::epsilon(format_ident!("epsilon"), vec![format_ident!("zeta")])]
    #[case::zeta(format_ident!("zeta"), vec![])]
    fn test_get_direct_downstream_jobs(
        all_jobs: JobConfigMap,
        #[case] task_id: syn::Ident,
        #[case] expected_job_ids: Vec<syn::Ident>,
    ) {
        let job = all_jobs.get(&task_id).unwrap();
        let expected = expected_job_ids
            .into_iter()
            .map(|id| all_jobs.get(&id).unwrap())
            .collect::<IndexSet<_>>();
        assert_eq!(get_direct_downstream_jobs(job, &all_jobs), expected);
    }
}
