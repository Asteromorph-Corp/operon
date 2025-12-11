use indexmap::IndexSet;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{
    dimension_metadata_ident, operon_ident, resolution_enum_ident, sender_ident, to_lit_str,
    to_pascal_case,
};

/// Generates the `on_receive_resolution` function for the implementation of the trait `JobSpec`.
///
/// # Example
/// ```rust, ignore
/// #[allow(unused_variables, clippy::match_single_binding)]
/// async fn on_receive_resolution(
///     &self,
///     client: operon::meta_storage::MetaClient<'_>,
///     peer_txs: &Self::PeerEventSenders,
///     resolution: schema::ResolutionEnum,
/// ) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
///     match resolution {
///         schema::ResolutionEnum::I(res) => Ok(client
///             .ticket(self.job_meta())
///             .explode::<_, 0usize>(metadata::dimension_i_meta(), res)
///             .await?),
///         schema::ResolutionEnum::J(res) => {
///             match peer_txs
///                 .to_epsilon
///                 .send(operon::scheduler::PeerEvent::Explosion(
///                     schema::ResolutionEnum::J(res),
///                 ))
///                 .await
///             {
///                 Ok(_) => {
///                     operon::log::trace!("`delta` sent peer event to `epsilon`: {resolution:?}")
///                 }
///                 Err(_) => {
///                     operon::log::trace!(
///                         "`epsilon`'s peer channel closed before handling `delta`'s {resolution:?}"
///                     )
///                 }
///             }
///             Ok(client
///                 .ticket(self.job_meta())
///                 .explode::<_, 1usize>(metadata::dimension_j_meta(), res)
///                 .await?)
///         }
///         schema::ResolutionEnum::K(res) => Ok(client
///             .ticket(self.job_meta())
///             .explode::<_, 2usize>(metadata::dimension_k_meta(), res)
///             .await?),
///         _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived(
///             "resolution",
///             "delta",
///         )),
///     }
/// }
/// ```
pub(super) fn fn_on_receive_resolution(
    job: &JobConfig,
    downstream_jobs: &IndexSet<&JobConfig>,
) -> syn::ImplItemFn {
    let operon = operon_ident();
    let res_enum_ident = resolution_enum_ident();
    let job_id = to_lit_str(&job.id);

    let explode_arms = job.dims.iter().enumerate().map(|(idx, dim)| -> syn::Arm {
        let variant_ident = to_pascal_case(dim);
        let dim_meta = dimension_metadata_ident(dim);
        let send_explosions = downstream_jobs.iter().flat_map(|downstream_job| {
            let cnt = downstream_job.from.iter().filter(|arg| arg.id == job.to && arg.over.contains(dim)).count();
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
                    .send(#operon::scheduler::PeerEvent::Explosion(schema::#res_enum_ident::#variant_ident(res), affected))
                    .await
                {
                    Ok(_) => #operon::log::trace!(#ok_msg),
                    Err(_) => #operon::log::trace!(#err_msg),
                }
            };
            std::iter::repeat_n(stmt, cnt)
        });

        parse_quote! {
            schema::#res_enum_ident::#variant_ident(res) => {
                let affected = client.ticket(self.job_meta()).explode::<_, #idx>(metadata::#dim_meta(), res).await?;
                #(#send_explosions)*
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
                _ => return Err(#operon::scheduler::SchedulerError::InvalidPeerEventReceived("resolution", #job_id)),
            }
            Ok(vec![])
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
    #[case::simple(job_delta(), "spec/spec/fn_on_receive_resolution.rs")]
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
