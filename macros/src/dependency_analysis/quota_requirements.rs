use indexmap::IndexSet;

use crate::configs::{DimensionConfig, DimensionConfigMap, JobConfig, JobConfigMap};
use crate::dependency_analysis::get_direct_upstream_jobs;

/// Get all dimensions that are required for the dependency quota calculation for the `target_job`.
pub fn get_quota_required_dims<'a>(
    target_job: &'a JobConfig,
    all_jobs: &'a JobConfigMap,
    all_dims: &'a DimensionConfigMap,
) -> IndexSet<&'a DimensionConfig> {
    get_direct_upstream_jobs(target_job, all_jobs)
        .into_iter()
        .flat_map(|upstream_job| {
            upstream_job
                .dims
                .iter()
                .filter(|dim| !target_job.dims.contains(dim))
        })
        .collect::<IndexSet<_>>()
        .into_iter()
        .map(|dim_id| {
            all_dims
                .get(dim_id)
                .unwrap_or_else(|| panic!("Dimension `{dim_id}` not found"))
        })
        .collect()
}
