use indexmap::IndexSet;

use crate::configs::{DimensionConfigMap, DimensionId};
use crate::{DimensionConfig, JobConfig, JobConfigMap};

/// Gett all dimensions that are required for the dependency quota calculation for the `target_job`.
pub fn get_quota_required_dims<'a>(
    target_job: &'a JobConfig,
    all_dims: &'a DimensionConfigMap,
) -> IndexSet<&'a DimensionConfig> {
    target_job
        .from
        .iter()
        .flat_map(|arg| arg.over.iter().filter(|dim| !target_job.dims.contains(dim)))
        .collect::<IndexSet<_>>()
        .into_iter()
        .map(|dim_id| {
            all_dims
                .get(dim_id)
                .unwrap_or_else(|| panic!("Dimension `{dim_id}` not found"))
        })
        .collect()
}

/// Gets all jobs that requires the `target_dim` in their dependencies quota calculation.
pub fn get_quota_requiring_jobs<'a>(
    target_dim: &'a DimensionId,
    all_jobs: &'a JobConfigMap,
) -> IndexSet<&'a JobConfig> {
    all_jobs
        .values()
        .filter(|job| {
            job.from.iter().any(|arg| {
                arg.over
                    .iter()
                    .any(|dim| dim == target_dim && !job.dims.contains(dim))
            })
        })
        .collect()
}
