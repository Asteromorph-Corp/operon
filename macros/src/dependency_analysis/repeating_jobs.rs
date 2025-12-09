use indexmap::IndexSet;

use crate::configs::{JobConfig, JobConfigMap};

/// Returns a set of job ids that are repeating jobs for the given target dimension.
pub fn get_jobs_repeating_on<'a>(
    target_dim: &'a syn::Ident,
    all_jobs: &'a JobConfigMap,
) -> IndexSet<&'a JobConfig> {
    all_jobs
        .values()
        .filter(|job| job.dims.contains(target_dim))
        .collect()
}

#[cfg(test)]
mod tests {
    use quote::format_ident;
    use rstest::rstest;

    use super::*;
    use crate::test_utils::simple_pipeline::all_jobs;

    #[rstest]
    #[case::i(format_ident!("i"), vec![format_ident!("beta"), format_ident!("gamma"), format_ident!("delta"), format_ident!("epsilon"), format_ident!("zeta")])]
    #[case::j(format_ident!("j"), vec![format_ident!("delta")])]
    #[case::k(format_ident!("k"), vec![format_ident!("delta"), format_ident!("epsilon")])]
    fn test_get_jobs_repeating_on(
        all_jobs: JobConfigMap,
        #[case] dim_id: syn::Ident,
        #[case] expected_job_ids: Vec<syn::Ident>,
    ) {
        let expected = expected_job_ids
            .into_iter()
            .map(|id| all_jobs.get(&id).unwrap())
            .collect::<IndexSet<_>>();
        assert_eq!(get_jobs_repeating_on(&dim_id, &all_jobs), expected);
    }
}
