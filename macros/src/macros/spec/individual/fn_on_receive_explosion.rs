use indexmap::IndexSet;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{operon_ident, raise_quota_ident, resolution_enum_ident, variant_ident};

pub(super) fn fn_on_receive_explosion(
    job: &JobConfig,
    upstream_jobs: &IndexSet<&JobConfig>,
) -> syn::ImplItemFn {
    let operon = operon_ident();
    let res_enum_ident = resolution_enum_ident();
    let job_id = &job.id;

    let arms = upstream_jobs
        .iter()
        .flat_map(|upstream_job| upstream_job.dims.iter().filter(|dim| !job.dims.contains(dim)))
        .collect::<IndexSet<_>>() // Deduplicate
        .into_iter()
        .map(|dim_id| -> syn::Arm {
            let variant_ident = variant_ident(dim_id);
            let raise_quota_fn_name = raise_quota_ident(job_id, dim_id);

            parse_quote! {
                schema::#res_enum_ident::#variant_ident(res) => Ok(
                    queries::#raise_quota_fn_name(client, &res).await?
                ),
            }
        });

    parse_quote! {
        #[allow(unused_variables, clippy::match_single_binding)]
        async fn on_receive_explosion(
            &self,
            client: #operon::meta_storage::MetaClient<'_>,
            resolution: schema::#res_enum_ident,
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
    use rstest::rstest;

    use super::*;
    use crate::configs::JobConfigMap;
    use crate::dependency_analysis::get_direct_upstream_jobs;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::all_jobs;

    #[rstest]
    #[case::simple("epsilon", "spec/fn_on_receive_explosion.rs")]
    fn test_fn_on_receive_explosion(
        all_jobs: JobConfigMap,
        #[case] job_id: &str,
        #[case] fixture_path: &str,
    ) {
        let job = all_jobs.get(job_id).unwrap();
        let upstream_jobs = get_direct_upstream_jobs(job, &all_jobs);
        let item = fn_on_receive_explosion(job, &upstream_jobs);
        assert_item_eq(&item, fixture_path);
    }
}
