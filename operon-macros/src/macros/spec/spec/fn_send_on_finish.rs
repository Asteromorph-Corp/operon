use indexmap::IndexSet;
use quote::format_ident;
use syn::parse_quote;

use crate::configs::JobConfig;
use crate::utils::{operon_ident, resolution_enum_ident, sender_ident, to_pascal_case};

/// Generates the `send_on_finish` function for the implementation of the trait `JobSpec`.
///
/// # Example
/// ```rust,ignore
/// async fn send_on_finish(
///     &self,
///     peer_txs: &Self::PeerEventSenders,
///     job: Self::Job,
///     resolution: Self::Resolution,
/// ) -> Result<(), operon::scheduler::SchedulerError> {
///     match peer_txs
///         .to_delta
///         .send(operon::scheduler::PeerEvent::Resolution(
///             schema::ResolutionEnum::J(resolution),
///         ))
///         .await
///     {
///         Ok(_) => operon::__private::tracing::trace!("`beta` sent peer event to `delta`: {resolution:?}"),
///         // Verbosity should be low here, since this can happen
///         // an arbitrary number of times
///         // if a descendant scheduler errored out.
///         Err(_) => operon::__private::tracing::trace!(
///             "`delta`'s peer channel closed before handling `beta`'s {resolution:?}"
///         ),
///     }
///     // out-dependencies (delta, epsilon)
///     match peer_txs
///         .to_delta
///         .send(operon::scheduler::PeerEvent::Job(schema::JobEnum::Beta(
///             job,
///         )))
///         .await
///     {
///         Ok(_) => operon::__private::tracing::trace!("`beta` sent peer event to `delta`: {job:?}"),
///         Err(_) => {
///             operon::__private::tracing::trace!("`delta`'s peer channel closed before handling `beta`'s {job:?}")
///         }
///     }
///     match peer_txs
///         .to_epsilon
///         .send(operon::scheduler::PeerEvent::Job(schema::JobEnum::Beta(
///             job,
///         )))
///         .await
///     {
///         Ok(_) => operon::__private::tracing::trace!("`beta` sent peer event to `epsilon`: {job:?}"),
///         Err(_) => {
///             operon::__private::tracing::trace!("`epsilon`'s peer channel closed before handling `beta`'s {job:?}")
///         }
///     }
///     Ok(())
/// }
/// ```
pub(super) fn fn_send_on_finish(
    job: &JobConfig,
    spawn_dim_repeating_jobs: &IndexSet<&JobConfig>,
    downstream_jobs: &IndexSet<&JobConfig>,
) -> syn::ImplItemFn {
    let operon = operon_ident();
    let resolution_enum_ident = resolution_enum_ident();

    let send_resolutions = spawn_dim_repeating_jobs
        .iter()
        .map(|repeating_job| -> syn::Expr {
            // TODO: remove unwrap
            let resolution_variant_ident = to_pascal_case(job.spawn_dim.as_ref().unwrap());
            let sender_ident = sender_ident(&repeating_job.id);
            let ok_msg = format!(
                "`{}` sent peer event to `{}`: {{resolution:?}}",
                job.id, repeating_job.id
            );
            let err_msg = format!(
                "`{}`'s peer channel closed before handling `{}`'s {{resolution:?}}",
                repeating_job.id, job.id
            );

            parse_quote! {
                match peer_txs
                    .#sender_ident
                    .send(#operon::scheduler::PeerEvent::Resolution(schema::#resolution_enum_ident::#resolution_variant_ident(resolution)))
                    .await
                {
                    Ok(_) => #operon::__private::tracing::trace!(#ok_msg),
                    Err(_) => #operon::__private::tracing::trace!(#err_msg),
                }
            }
        });

    let send_jobs = downstream_jobs.iter().map(|downstream_job| -> syn::Expr {
        let job_variant_ident = to_pascal_case(&format_ident!("{}", job.id));
        let sender_ident = sender_ident(&downstream_job.id);
        let ok_msg = format!(
            "`{}` sent peer event to `{}`: {{job:?}}",
            job.id, downstream_job.id
        );
        let err_msg = format!(
            "`{}`'s peer channel closed before handling `{}`'s {{job:?}}",
            downstream_job.id, job.id
        );

        parse_quote! {
            match peer_txs
                .#sender_ident
                .send(#operon::scheduler::PeerEvent::Job(schema::JobEnum::#job_variant_ident(job)))
                .await
            {
                Ok(_) => #operon::__private::tracing::trace!(#ok_msg),
                Err(_) => #operon::__private::tracing::trace!(#err_msg),
            }
        }
    });

    parse_quote! {
        async fn send_on_finish(
            &self,
            peer_txs: &Self::PeerEventSenders,
            job: Self::Job,
            resolution: Self::Resolution,
        ) -> Result<(), #operon::scheduler::SchedulerError> {
            #(#send_resolutions)*
            #(#send_jobs)*
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::configs::JobConfigMap;
    use crate::dependency_analysis::{get_direct_downstream_jobs, get_jobs_repeating_on};
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_jobs, job_beta};

    #[rstest]
    #[case::simple(job_beta(), "spec/spec/fn_send_on_finish.rs")]
    fn test_fn_send_on_finish(
        all_jobs: JobConfigMap,
        #[case] job: JobConfig,
        #[case] fixture_path: &str,
    ) {
        let spawn_dim_repeating_jobs = job
            .spawn_dim
            .as_ref()
            .map(|dim| get_jobs_repeating_on(dim, &all_jobs))
            .unwrap_or_default();
        let downstream_jobs = get_direct_downstream_jobs(&job, &all_jobs);

        let item = fn_send_on_finish(&job, &spawn_dim_repeating_jobs, &downstream_jobs);
        assert_item_eq(&item, fixture_path);
    }
}
