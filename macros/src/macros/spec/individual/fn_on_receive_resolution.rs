use indexmap::IndexSet;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{
    explode_ident, operon_ident, resolution_enum_ident, sender_ident, variant_ident,
};

/// Generates the `on_receive_resolution` function for the implementation of the trait `JobSpec`.
///
/// Example:
/// ```rust, ignore
/// #[allow(unused_variables, clippy::match_single_binding)]
/// async fn on_receive_resolution(
///     &self,
///     client: operon::meta_storage::MetaClient<'_>,
///     resolution: schema::ResolutionEnum,
/// ) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
///     match resolution {
///         schema::ResolutionEnum::I(res) => Ok(
///             queries::explode_delta_i(client, &res).await?
///         ),
///         schema::ResolutionEnum::J(res) => Ok(
///             queries::explode_delta_j(client, &res).await?
///         ),
///         schema::ResolutionEnum::K(res) => Ok(
///             queries::explode_delta_k(client, &res).await?
///         ),
///         _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived("resolution", "delta")),
///     }
/// }
/// ```
pub(super) fn fn_on_receive_resolution(
    job: &JobConfig,
    downstream_jobs: &IndexSet<&JobConfig>,
) -> syn::ImplItemFn {
    let operon = operon_ident();
    let res_enum_ident = resolution_enum_ident();
    let job_id = &job.id;

    let explode_arms = job.dims.iter().map(|dim| -> syn::Arm {
        let variant_ident = variant_ident(dim);
        let explode_fn_name = explode_ident(job_id, dim);
        let send_explosions = downstream_jobs.iter().filter_map(|downstream_job| {
            if downstream_job.dims.contains(dim) {
                return None;
            }
            let sender_ident = sender_ident(&downstream_job.id);
            let ok_msg = format!(
                "`{}` sent peer event to `{}`: {{resolution:?}}",
                job.id, downstream_job.id
            );
            let err_msg = format!(
                "`{}`'s peer channel closed before handling `{}`'s {{resolution:?}}",
                downstream_job.id, job.id
            );

            let stmt: syn::Stmt = parse_quote! {
                match peer_txs
                    .#sender_ident
                    .send(#operon::scheduler::PeerEvent::Explosion(res.into()))
                    .await
                {
                    Ok(_) => #operon::log::trace!(#ok_msg),
                    Err(_) => #operon::log::trace!(#err_msg),
                }
            };
            Some(stmt)
        });

        parse_quote! {
            schema::#res_enum_ident::#variant_ident(res) => {
                #(#send_explosions)*
                Ok(queries::#explode_fn_name(client, &res).await?)
            },
        }
    });

    parse_quote! {
        #[allow(unused_variables, clippy::match_single_binding)]
        async fn on_receive_resolution(
            &self,
            client: #operon::meta_storage::MetaClient<'_>,
            peer_txs: &Self::PeerEventSenders,
            resolution: schema::#res_enum_ident,
        ) -> Result<Vec<Self::Ticket>, #operon::scheduler::SchedulerError> {
            match resolution {
                #(#explode_arms)*
                _ => Err(#operon::scheduler::SchedulerError::InvalidPeerEventReceived("resolution", #job_id)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::configs::JobConfigMap;
    use crate::dependency_analysis::get_direct_downstream_jobs;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_jobs, job_delta};

    #[rstest]
    #[case::simple(job_delta(), "spec/fn_on_receive_resolution.rs")]
    fn test_fn_on_receive_resolution(
        all_jobs: JobConfigMap,
        #[case] job: JobConfig,
        #[case] fixture_path: &str,
    ) {
        let downstream_jobs = get_direct_downstream_jobs(&job, &all_jobs);
        let item = fn_on_receive_resolution(&job, &downstream_jobs);
        assert_item_eq(&item, fixture_path);
    }
}
