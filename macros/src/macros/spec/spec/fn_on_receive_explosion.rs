use indexmap::IndexSet;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{
    dimension_metadata_ident, operon_ident, resolution_enum_ident, to_lit_str, to_pascal_case,
};

/// Generates the `on_receive_explosion` function for the implementation of the trait `JobSpec`.
///
/// # Example
/// ```rust,ignore
/// #[allow(unused_variables, clippy::match_single_binding)]
/// async fn on_receive_explosion(
///     &self,
///     client: operon::meta_storage::MetaClient<'_>,
///     resolution: schema::ResolutionEnum,
/// ) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
///     match resolution {
///         schema::ResolutionEnum::J(res) => Ok(client
///             .ticket(self.job_meta())
///             .raise_deps_quota(metadata::dimension_j_meta(), res)
///             .await?),
///         _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived(
///             "explosion",
///             "epsilon",
///         )),
///     }
/// }
/// ```
pub(super) fn fn_on_receive_explosion(
    job: &JobConfig,
    upstream_jobs: &IndexSet<&JobConfig>,
) -> syn::ImplItemFn {
    let operon = operon_ident();
    let res_enum_ident = resolution_enum_ident();
    let job_id = to_lit_str(&job.id);

    let arms = upstream_jobs
        .iter()
        .flat_map(|upstream_job| upstream_job.dims.iter().filter(|dim| !job.dims.contains(dim)))
        .collect::<IndexSet<_>>() // Deduplicate
        .into_iter()
        .map(|dim_id| -> syn::Arm {
            let variant_ident = to_pascal_case(dim_id);
            let dim_meta = dimension_metadata_ident(dim_id);

            parse_quote! {
                schema::#res_enum_ident::#variant_ident(res) => Ok(
                    client.ticket(self.job_meta())
                        .raise_deps_quota(metadata::#dim_meta(), res, affected)
                        .await?
                ),
            }
        });

    parse_quote! {
        #[allow(unused_variables, clippy::match_single_binding)]
        async fn on_receive_explosion(
            &self,
            client: #operon::meta_storage::MetaClient<'_>,
            resolution: schema::#res_enum_ident,
            affected: usize,
        ) -> Result<Vec<Self::Ticket>, #operon::scheduler::SchedulerError> {
            match resolution {
                #(#arms)*
                _ => Err(#operon::scheduler::SchedulerError::InvalidPeerEventReceived("explosion", #job_id)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::format_ident;
    use rstest::rstest;

    use super::*;
    use crate::configs::JobConfigMap;
    use crate::dependency_analysis::get_direct_upstream_jobs;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::all_jobs;

    #[rstest]
    #[case::simple(format_ident!("epsilon"), "spec/spec/fn_on_receive_explosion.rs")]
    fn test_fn_on_receive_explosion(
        all_jobs: JobConfigMap,
        #[case] job_id: syn::Ident,
        #[case] fixture_path: &str,
    ) {
        let job = all_jobs.get(&job_id).unwrap();
        let upstream_jobs = get_direct_upstream_jobs(job, &all_jobs);
        let item = fn_on_receive_explosion(job, &upstream_jobs);
        assert_item_eq(&item, fixture_path);
    }
}
